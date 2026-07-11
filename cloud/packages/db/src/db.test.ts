import { eq } from 'drizzle-orm';
import { expect, test } from 'vitest';
import { organization, user, webhookSubscriptions } from './schema';
import { createTestDb } from './testing';

test('organization round-trip with null master routing by default', async () => {
  const { db } = await createTestDb();

  await db
    .insert(organization)
    .values({ id: 'org_1', name: 'Acme', slug: 'acme', createdAt: new Date() });

  const rows = await db.select().from(organization).where(eq(organization.slug, 'acme'));

  expect(rows).toHaveLength(1);
  expect(rows[0]?.name).toBe('Acme');
  expect(rows[0]?.masterUrl).toBeNull();
  expect(rows[0]?.masterAdminToken).toBeNull();
});

test('webhook subscription references org + creator and stores typed scope/events', async () => {
  const { db } = await createTestDb();

  await db.insert(user).values({ id: 'user_1', name: 'Ada', email: 'ada@example.com' });
  await db
    .insert(organization)
    .values({ id: 'org_2', name: 'Beta', slug: 'beta', createdAt: new Date() });
  await db.insert(webhookSubscriptions).values({
    id: 'wh_1',
    orgId: 'org_2',
    scope: 'server',
    target: 'agent-123',
    url: 'https://hooks.example.com/x',
    secret: 's3cr3t',
    events: ['alert.cpu', 'alert.mem'],
    createdBy: 'user_1',
  });

  const [sub] = await db.select().from(webhookSubscriptions);

  expect(sub?.scope).toBe('server');
  expect(sub?.events).toEqual(['alert.cpu', 'alert.mem']);
  expect(sub?.active).toBe(true);
  expect(sub?.createdBy).toBe('user_1');
});
