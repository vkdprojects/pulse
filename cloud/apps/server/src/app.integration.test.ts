import { createAuth } from '@vixpi/auth';
import type { MeResponse } from '@vixpi/contracts';
import type { Db } from '@vixpi/db';
import { createTestDb } from '@vixpi/db/testing';
import { expect, test } from 'vitest';
import { createApp } from './app';

async function setup() {
  const { db } = await createTestDb();
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
    version: '1.2.3',
  });
  return { app };
}

async function signUp(app: Awaited<ReturnType<typeof setup>>['app']) {
  const res = await app.handle(
    new Request('http://localhost/api/auth/sign-up/email', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        email: 'ada@example.com',
        password: 'sup3r-secret-pw',
        name: 'Ada Lovelace',
      }),
    }),
  );
  const cookie = (res.headers.get('set-cookie') ?? '').split(';')[0] ?? '';
  return cookie;
}

test('/v1/me is 401 without a session', async () => {
  const { app } = await setup();
  const res = await app.handle(new Request('http://localhost/v1/me'));
  expect(res.status).toBe(401);
  expect(await res.json()).toMatchObject({ error: { code: 'unauthorized' } });
});

test('/v1/me returns the signed-up user (no active org yet)', async () => {
  const { app } = await setup();
  const cookie = await signUp(app);
  expect(cookie).not.toBe('');

  const res = await app.handle(new Request('http://localhost/v1/me', { headers: { cookie } }));
  expect(res.status).toBe(200);
  const body = (await res.json()) as MeResponse;
  expect(body.user.email).toBe('ada@example.com');
  expect(body.org).toBeNull();
});

test('/v1/me reflects the active org and owner role after creating one', async () => {
  const { app } = await setup();
  const cookie = await signUp(app);

  const created = await app.handle(
    new Request('http://localhost/api/auth/organization/create', {
      method: 'POST',
      headers: { 'content-type': 'application/json', cookie },
      body: JSON.stringify({ name: 'Acme', slug: 'acme' }),
    }),
  );
  expect(created.status).toBe(200);
  const createdOrg = (await created.json()) as { id: string };

  // Creating an org does not auto-activate it in the session — set it active explicitly.
  const activated = await app.handle(
    new Request('http://localhost/api/auth/organization/set-active', {
      method: 'POST',
      headers: { 'content-type': 'application/json', cookie },
      body: JSON.stringify({ organizationId: createdOrg.id }),
    }),
  );
  expect(activated.status).toBe(200);

  const res = await app.handle(new Request('http://localhost/v1/me', { headers: { cookie } }));
  const body = (await res.json()) as MeResponse;
  expect(body.org?.slug).toBe('acme');
  expect(body.org?.role).toBe('owner');
  expect(body.org?.hasMaster).toBe(false);
});
