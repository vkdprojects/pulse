import type { Db } from '@vixpi/db';
import { member } from '@vixpi/db';
import { createTestDb } from '@vixpi/db/testing';
import { expect, test } from 'vitest';
import { createAuth } from './index';

async function setup() {
  const { db } = await createTestDb();
  const auth = createAuth({
    // PGlite is a real Postgres; the drizzle adapter accepts it at runtime. Cast for the type.
    db: db as unknown as Db,
    secret: 'integration-test-secret-32-characters',
    baseUrl: 'http://localhost:3000',
    trustedOrigins: ['http://localhost:5173'],
  });
  return { db, auth };
}

/** Take the `name=value` of the session cookie out of a Set-Cookie header. */
function sessionCookie(headers: Headers): string {
  const setCookie = headers.get('set-cookie') ?? '';
  return setCookie.split(';')[0] ?? '';
}

test('sign up creates a user and yields a valid session', async () => {
  const { auth } = await setup();

  const { headers } = await auth.api.signUpEmail({
    returnHeaders: true,
    body: { email: 'ada@example.com', password: 'sup3r-secret-pw', name: 'Ada Lovelace' },
  });

  const cookie = sessionCookie(headers);
  expect(cookie).not.toBe('');

  const session = await auth.api.getSession({ headers: { cookie } });
  expect(session?.user.email).toBe('ada@example.com');
});

test('a signed-up user can create an org and becomes its owner', async () => {
  const { db, auth } = await setup();

  const signUp = await auth.api.signUpEmail({
    returnHeaders: true,
    body: { email: 'grace@example.com', password: 'another-secret-pw', name: 'Grace Hopper' },
  });
  const cookie = sessionCookie(signUp.headers);

  const org = await auth.api.createOrganization({
    headers: { cookie },
    body: { name: 'Hopper Labs', slug: 'hopper-labs' },
  });
  expect(org?.slug).toBe('hopper-labs');

  const members = await db.select().from(member);
  expect(members).toHaveLength(1);
  expect(members[0]?.role).toBe('owner');

  // Master routing starts unset — assigned server-side during provisioning.
  expect(org?.masterUrl ?? null).toBeNull();
});

test('unauthenticated getSession returns null', async () => {
  const { auth } = await setup();
  const session = await auth.api.getSession({ headers: {} });
  expect(session).toBeNull();
});
