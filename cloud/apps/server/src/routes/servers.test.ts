import { createAuth } from '@vixpi/auth';
import type { FetchLike } from '@vixpi/contracts';
import { auditLog, type Db, organization } from '@vixpi/db';
import { createTestDb } from '@vixpi/db/testing';
import { eq } from 'drizzle-orm';
import { expect, test } from 'vitest';
import { createApp } from '../app';

function jsonResponse(value: unknown, status = 200): Response {
  return new Response(JSON.stringify(value), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}

/** Stub pulse-master: canned overview + exec, echoes the bearer token it received. */
function makeMasterFetch(): { fetch: FetchLike; lastAuth: () => string | null } {
  let lastAuth: string | null = null;
  const fetch: FetchLike = async (input, init) => {
    const url = String(input);
    const method = init?.method ?? 'GET';
    lastAuth = new Headers(init?.headers).get('authorization');

    if (url.endsWith('/api/overview')) {
      return jsonResponse({
        total_agents: 1,
        online: 1,
        offline: 0,
        agents: [{ id: 'a1', hostname: 'web-1', status: 'online' }],
      });
    }
    if (url.includes('/exec') && method === 'POST') {
      return jsonResponse({
        agent_id: 'a1',
        exit_code: 0,
        stdout: 'hello',
        stderr: '',
        timed_out: false,
      });
    }
    return jsonResponse({ error: 'not found' }, 404);
  };
  return { fetch, lastAuth: () => lastAuth };
}

async function setup() {
  const { db } = await createTestDb();
  const master = makeMasterFetch();
  const auth = createAuth({
    db: db as unknown as Db,
    secret: 'integration-test-secret-32-characters',
    baseUrl: 'http://localhost:3000',
    trustedOrigins: ['http://localhost:5173'],
  });
  const app = createApp({
    auth,
    db: db as unknown as Db,
    corsOrigins: ['http://localhost:5173'],
    masterFetch: master.fetch,
  });
  return { db: db as unknown as Db, app, master };
}

async function signUpWithActiveOrg(app: Awaited<ReturnType<typeof setup>>['app']) {
  const su = await app.handle(
    new Request('http://localhost/api/auth/sign-up/email', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ email: 'op@example.com', password: 'sup3r-secret-pw', name: 'Op' }),
    }),
  );
  const cookie = (su.headers.get('set-cookie') ?? '').split(';')[0] ?? '';

  const created = await app.handle(
    new Request('http://localhost/api/auth/organization/create', {
      method: 'POST',
      headers: { 'content-type': 'application/json', cookie },
      body: JSON.stringify({ name: 'Acme', slug: 'acme' }),
    }),
  );
  const org = (await created.json()) as { id: string };
  await app.handle(
    new Request('http://localhost/api/auth/organization/set-active', {
      method: 'POST',
      headers: { 'content-type': 'application/json', cookie },
      body: JSON.stringify({ organizationId: org.id }),
    }),
  );
  return { cookie, orgId: org.id };
}

test('GET /v1/servers is 401 without a session', async () => {
  const { app } = await setup();
  const res = await app.handle(new Request('http://localhost/v1/servers'));
  expect(res.status).toBe(401);
});

test('GET /v1/servers is 409 before a master is provisioned', async () => {
  const { app } = await setup();
  const { cookie } = await signUpWithActiveOrg(app);
  const res = await app.handle(new Request('http://localhost/v1/servers', { headers: { cookie } }));
  expect(res.status).toBe(409);
  expect(await res.json()).toMatchObject({ error: { code: 'no_master' } });
});

test('GET /v1/servers proxies the org master once provisioned', async () => {
  const { app, db, master } = await setup();
  const { cookie, orgId } = await signUpWithActiveOrg(app);

  await db
    .update(organization)
    .set({ masterUrl: 'http://master.test', masterAdminToken: 'org-token' })
    .where(eq(organization.id, orgId));

  const res = await app.handle(new Request('http://localhost/v1/servers', { headers: { cookie } }));
  expect(res.status).toBe(200);
  const body = (await res.json()) as { totalAgents: number; agents: { hostname: string }[] };
  expect(body.totalAgents).toBe(1);
  expect(body.agents[0]?.hostname).toBe('web-1');
  expect(master.lastAuth()).toBe('Bearer org-token');
});

test('POST /v1/servers/:id/exec runs and writes an audit row', async () => {
  const { app, db } = await setup();
  const { cookie, orgId } = await signUpWithActiveOrg(app);
  await db
    .update(organization)
    .set({ masterUrl: 'http://master.test', masterAdminToken: 'org-token' })
    .where(eq(organization.id, orgId));

  const res = await app.handle(
    new Request('http://localhost/v1/servers/a1/exec', {
      method: 'POST',
      headers: { 'content-type': 'application/json', cookie },
      body: JSON.stringify({ command: 'uptime' }),
    }),
  );
  expect(res.status).toBe(200);
  expect((await res.json()) as { exitCode: number }).toMatchObject({ exitCode: 0 });

  const audits = await db.select().from(auditLog);
  expect(audits).toHaveLength(1);
  expect(audits[0]?.action).toBe('server.exec');
  expect(audits[0]?.targetId).toBe('a1');
  expect(audits[0]?.metadata).toMatchObject({ command: 'uptime' });
});
