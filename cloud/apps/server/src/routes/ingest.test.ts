import type { FetchLike } from '@vixpi/contracts';
import { webhookDeliveries } from '@vixpi/db';
import { expect, test } from 'vitest';
import { makeTestApp, signUpWithActiveOrg } from '../testkit';

const INGEST_SECRET = 'ingest-secret-value';

function captureFetch(): { fetch: FetchLike; calls: () => number } {
  let calls = 0;
  const fetch: FetchLike = async () => {
    calls += 1;
    return new Response(null, { status: 200 });
  };
  return { fetch, calls: () => calls };
}

async function createOrgSub(app: Awaited<ReturnType<typeof makeTestApp>>['app'], cookie: string) {
  await app.handle(
    new Request('http://localhost/v1/webhooks', {
      method: 'POST',
      headers: { 'content-type': 'application/json', cookie },
      body: JSON.stringify({ scope: 'org', url: 'https://hook.test/a', events: ['alert.cpu'] }),
    }),
  );
}

test('ingest rejects a bad secret with 401', async () => {
  const { app } = await makeTestApp({ alertIngestSecret: INGEST_SECRET });
  const { orgId } = await signUpWithActiveOrg(app);

  const res = await app.handle(
    new Request(`http://localhost/v1/internal/alerts/${orgId}`, {
      method: 'POST',
      headers: { 'content-type': 'application/json', 'x-ingest-secret': 'wrong' },
      body: JSON.stringify({ event: 'alert.cpu' }),
    }),
  );
  expect(res.status).toBe(401);
});

test('ingest is 503 when no secret is configured', async () => {
  const { app } = await makeTestApp();
  const res = await app.handle(
    new Request('http://localhost/v1/internal/alerts/whatever', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ event: 'alert.cpu' }),
    }),
  );
  expect(res.status).toBe(503);
});

test('a valid alert fans out to matching subscriptions and logs deliveries', async () => {
  const capture = captureFetch();
  const { app, db } = await makeTestApp({
    alertIngestSecret: INGEST_SECRET,
    webhookFetch: capture.fetch,
  });
  const { cookie, orgId } = await signUpWithActiveOrg(app);
  await createOrgSub(app, cookie);

  const res = await app.handle(
    new Request(`http://localhost/v1/internal/alerts/${orgId}`, {
      method: 'POST',
      headers: { 'content-type': 'application/json', 'x-ingest-secret': INGEST_SECRET },
      body: JSON.stringify({ event: 'alert.cpu', agentId: 'a1', data: { pct: 96 } }),
    }),
  );
  expect(res.status).toBe(200);
  expect((await res.json()) as { dispatched: number }).toMatchObject({ dispatched: 1 });
  expect(capture.calls()).toBe(1);

  const deliveries = await db.select().from(webhookDeliveries);
  expect(deliveries).toHaveLength(1);
  expect(deliveries[0]?.event).toBe('alert.cpu');
  expect(deliveries[0]?.status).toBe('success');
});
