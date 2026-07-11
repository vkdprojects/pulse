import type { FetchLike } from '@vixpi/contracts';
import { webhookDeliveries } from '@vixpi/db';
import { expect, test } from 'vitest';
import { makeTestApp, signUpWithActiveOrg } from '../testkit';

type App = Awaited<ReturnType<typeof makeTestApp>>['app'];

function captureFetch(status = 204): { fetch: FetchLike; calls: () => number } {
  let calls = 0;
  const fetch: FetchLike = async () => {
    calls += 1;
    return new Response(null, { status });
  };
  return { fetch, calls: () => calls };
}

async function jsonPost(app: App, path: string, cookie: string, body: unknown) {
  return app.handle(
    new Request(`http://localhost${path}`, {
      method: 'POST',
      headers: { 'content-type': 'application/json', cookie },
      body: JSON.stringify(body),
    }),
  );
}

test('create returns the secret once; list never exposes it', async () => {
  const { app } = await makeTestApp();
  const { cookie } = await signUpWithActiveOrg(app);

  const created = await jsonPost(app, '/v1/webhooks', cookie, {
    scope: 'org',
    url: 'https://hook.test/a',
    events: ['alert.cpu'],
  });
  expect(created.status).toBe(200);
  const sub = (await created.json()) as { id: string; secret: string };
  expect(sub.secret).toMatch(/^[0-9a-f]{48}$/);

  const list = await app.handle(
    new Request('http://localhost/v1/webhooks', { headers: { cookie } }),
  );
  const body = (await list.json()) as { webhooks: Array<Record<string, unknown>> };
  expect(body.webhooks).toHaveLength(1);
  expect(body.webhooks[0]).not.toHaveProperty('secret');
});

test('unauthenticated is 401', async () => {
  const { app } = await makeTestApp();
  const res = await app.handle(new Request('http://localhost/v1/webhooks'));
  expect(res.status).toBe(401);
});

test('test-send delivers to the endpoint and records a delivery', async () => {
  const capture = captureFetch(200);
  const { app, db } = await makeTestApp({ webhookFetch: capture.fetch });
  const { cookie } = await signUpWithActiveOrg(app);

  const created = await jsonPost(app, '/v1/webhooks', cookie, {
    scope: 'org',
    url: 'https://hook.test/a',
    events: ['alert.cpu'],
  });
  const { id } = (await created.json()) as { id: string };

  const test = await jsonPost(app, `/v1/webhooks/${id}/test`, cookie, {});
  expect(test.status).toBe(200);
  expect(capture.calls()).toBe(1);

  const deliveries = await db.select().from(webhookDeliveries);
  expect(deliveries).toHaveLength(1);
  expect(deliveries[0]?.event).toBe('test.ping');
  expect(deliveries[0]?.status).toBe('success');
});

test('delete removes the subscription (scoped to the org)', async () => {
  const { app } = await makeTestApp();
  const { cookie } = await signUpWithActiveOrg(app);
  const created = await jsonPost(app, '/v1/webhooks', cookie, {
    scope: 'server',
    target: 'a1',
    url: 'https://hook.test/a',
    events: ['alert.cpu'],
  });
  const { id } = (await created.json()) as { id: string };

  const del = await app.handle(
    new Request(`http://localhost/v1/webhooks/${id}`, { method: 'DELETE', headers: { cookie } }),
  );
  expect(del.status).toBe(200);

  const list = await app.handle(
    new Request('http://localhost/v1/webhooks', { headers: { cookie } }),
  );
  expect((await list.json()) as { webhooks: unknown[] }).toMatchObject({ webhooks: [] });
});
