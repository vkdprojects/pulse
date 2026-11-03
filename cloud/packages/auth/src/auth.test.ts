import { createDb } from '@vixpi/db';
import { expect, test } from 'vitest';
import { createAuth } from './index';

test('createAuth builds a better-auth instance with a request handler', () => {
  // No DB connection is opened at construction time; the client is lazy.
  const { db } = createDb('postgres://pulse:pulse@localhost:5432/pulse');

  const auth = createAuth({
    db,
    secret: 'test-secret-please-change',
    baseUrl: 'http://localhost:3000',
    trustedOrigins: ['http://localhost:5173'],
  });

  expect(typeof auth.handler).toBe('function');
  expect(auth.api).toBeTypeOf('object');
});
