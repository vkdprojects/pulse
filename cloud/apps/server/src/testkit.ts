import { createAuth } from '@vixpi/auth';
import type { Db } from '@vixpi/db';
import { createTestDb } from '@vixpi/db/testing';
import { type App, type AppDeps, createApp } from './app';

/** Build an app backed by an in-process Postgres (PGlite) for integration tests. */
export async function makeTestApp(extra?: Partial<AppDeps>): Promise<{ app: App; db: Db }> {
  const { db: raw } = await createTestDb();
  const db = raw as unknown as Db;
  const auth = createAuth({
    db,
    secret: 'integration-test-secret-32-characters',
    baseUrl: 'http://localhost:3000',
    trustedOrigins: ['http://localhost:5173'],
  });
  const app = createApp({ auth, db, corsOrigins: ['http://localhost:5173'], ...extra });
  return { app, db };
}

/** Sign up a user and give them an active organization; returns the session cookie + org id. */
export async function signUpWithActiveOrg(
  app: App,
  email = 'op@example.com',
): Promise<{ cookie: string; orgId: string }> {
  const su = await app.handle(
    new Request('http://localhost/api/auth/sign-up/email', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ email, password: 'sup3r-secret-pw', name: 'Op' }),
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
