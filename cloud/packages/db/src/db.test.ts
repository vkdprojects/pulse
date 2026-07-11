import { eq } from 'drizzle-orm';
import { expect, test } from 'vitest';
import { organizations, webhookSubscriptions } from './schema';
import { createTestDb } from './testing';

test('organizations round-trip with defaults', async () => {
  const { db } = await createTestDb();

  await db.insert(organizations).values({
    id: 'org_1',
    name: 'Acme',
    slug: 'acme',
    masterUrl: 'http://master:8080',
  });

  const rows = await db.select().from(organizations).where(eq(organizations.slug, 'acme'));

  expect(rows).toHaveLength(1);
  expect(rows[0]?.name).toBe('Acme');
  expect(rows[0]?.masterAdminToken).toBeNull();
  expect(rows[0]?.createdAt).toBeInstanceOf(Date);
});

test('webhook subscription stores typed scope and json events', async () => {
  const { db } = await createTestDb();

  await db.insert(organizations).values({ id: 'org_2', name: 'Beta', slug: 'beta' });
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
});
