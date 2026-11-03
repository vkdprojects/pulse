import type { FetchLike } from '@vixpi/contracts';
import { type Db, organization, user, webhookDeliveries, webhookSubscriptions } from '@vixpi/db';
import { createTestDb } from '@vixpi/db/testing';
import { expect, test } from 'vitest';
import { dispatch, matchSubscriptions } from './dispatch';
import { verifySignature } from './sign';

async function seed() {
  const { db: raw } = await createTestDb();
  const db = raw as unknown as Db;
  await db.insert(user).values({ id: 'u1', name: 'U', email: 'u@e.com' });
  await db.insert(organization).values({ id: 'o1', name: 'O', slug: 'o', createdAt: new Date() });
  return db;
}

async function addSub(
  db: Db,
  over: Partial<typeof webhookSubscriptions.$inferInsert> & { id: string },
) {
  await db.insert(webhookSubscriptions).values({
    orgId: 'o1',
    scope: 'org',
    url: 'https://hook.test/x',
    secret: 'sub-secret',
    events: ['alert.cpu'],
    createdBy: 'u1',
    ...over,
  });
}

test('matchSubscriptions honours scope and event selectors', async () => {
  const db = await seed();
  await addSub(db, { id: 'org-sub', scope: 'org', events: ['alert.cpu'] });
  await addSub(db, { id: 'srv-a', scope: 'server', target: 'a1', events: ['alert.cpu'] });
  await addSub(db, { id: 'srv-b', scope: 'server', target: 'b2', events: ['alert.cpu'] });
  await addSub(db, { id: 'wrong-event', scope: 'org', events: ['alert.mem'] });
  await addSub(db, { id: 'inactive', scope: 'org', events: ['alert.cpu'], active: false });

  const matched = await matchSubscriptions(db, 'o1', { event: 'alert.cpu', agentId: 'a1' });
  const ids = matched.map((s) => s.id).sort();
  expect(ids).toEqual(['org-sub', 'srv-a']);
});

test('dispatch signs each payload and records a delivery per subscription', async () => {
  const db = await seed();
  await addSub(db, { id: 'org-sub', scope: 'org', secret: 'shhh', events: ['alert.cpu'] });

  let seenSig: string | null = null;
  let seenBody: string | null = null;
  const fetchMock: FetchLike = async (_url, init) => {
    seenSig = new Headers(init?.headers).get('x-vixpi-signature');
    seenBody = String(init?.body);
    return new Response(null, { status: 204 });
  };

  const outcomes = await dispatch(db, 'o1', { event: 'alert.cpu', data: { pct: 95 } }, fetchMock);

  expect(outcomes).toHaveLength(1);
  expect(outcomes[0]?.status).toBe('success');
  expect(seenSig).not.toBeNull();
  expect(verifySignature('shhh', seenBody ?? '', seenSig ?? '')).toBe(true);

  const deliveries = await db.select().from(webhookDeliveries);
  expect(deliveries).toHaveLength(1);
  expect(deliveries[0]?.status).toBe('success');
  expect(deliveries[0]?.responseStatus).toBe(204);
});

test('a failing endpoint is recorded as a failed delivery', async () => {
  const db = await seed();
  await addSub(db, { id: 'org-sub', scope: 'org', events: ['alert.cpu'] });

  const fetchMock: FetchLike = async () => new Response('nope', { status: 500 });
  const outcomes = await dispatch(db, 'o1', { event: 'alert.cpu' }, fetchMock);

  expect(outcomes[0]?.status).toBe('failed');
  const [delivery] = await db.select().from(webhookDeliveries);
  expect(delivery?.status).toBe('failed');
  expect(delivery?.error).toBe('HTTP 500');
});
