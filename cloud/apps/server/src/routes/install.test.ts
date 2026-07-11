import type { FetchLike } from '@vixpi/contracts';
import { organization } from '@vixpi/db';
import { eq } from 'drizzle-orm';
import { expect, test } from 'vitest';
import { makeTestApp, signUpWithActiveOrg } from '../testkit';

const masterFetch: FetchLike = async (input) => {
  if (String(input).endsWith('/api/register-token')) {
    return new Response(JSON.stringify({ register_token: 'reg-abc123' }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    });
  }
  return new Response('not found', { status: 404 });
};

test('GET /v1/servers/install is 409 before a master is provisioned', async () => {
  const { app } = await makeTestApp({ masterFetch });
  const { cookie } = await signUpWithActiveOrg(app);
  const res = await app.handle(
    new Request('http://localhost/v1/servers/install', { headers: { cookie } }),
  );
  expect(res.status).toBe(409);
});

test('install returns a command carrying the register token and gRPC endpoint', async () => {
  const { app, db } = await makeTestApp({ masterFetch });
  const { cookie, orgId } = await signUpWithActiveOrg(app);
  await db
    .update(organization)
    .set({ masterUrl: 'http://master.test:8080', masterAdminToken: 'org-token' })
    .where(eq(organization.id, orgId));

  const res = await app.handle(
    new Request('http://localhost/v1/servers/install', { headers: { cookie } }),
  );
  expect(res.status).toBe(200);
  const body = (await res.json()) as {
    command: string;
    registerToken: string;
    masterEndpoint: string;
  };
  expect(body.registerToken).toBe('reg-abc123');
  expect(body.masterEndpoint).toBe('https://master.test:50051');
  expect(body.command).toContain('PULSE_TOKEN=reg-abc123');
  expect(body.command).toContain('PULSE_MASTER=https://master.test:50051');
});
