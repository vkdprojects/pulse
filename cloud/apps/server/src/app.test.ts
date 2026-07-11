import { createAuth } from '@vixpi/auth';
import { createDb } from '@vixpi/db';
import { expect, test } from 'vitest';
import { createApp } from './app';

function makeApp() {
  const { db } = createDb('postgres://pulse:pulse@localhost:5432/pulse');
  const auth = createAuth({
    db,
    secret: 'test-secret-please-change',
    baseUrl: 'http://localhost:3000',
    trustedOrigins: ['http://localhost:5173'],
  });
  return createApp({ auth, db, corsOrigins: ['http://localhost:5173'], version: '9.9.9' });
}

test('GET /health returns ok with security headers', async () => {
  const app = makeApp();
  const res = await app.handle(new Request('http://localhost/health'));

  expect(res.status).toBe(200);
  expect(res.headers.get('x-content-type-options')).toBe('nosniff');
  expect(res.headers.get('x-frame-options')).toBe('DENY');
  expect(await res.json()).toEqual({ status: 'ok', service: 'vixpi-pulse-api', version: '9.9.9' });
});

test('GET /v1/ responds', async () => {
  const app = makeApp();
  const res = await app.handle(new Request('http://localhost/v1/'));
  expect(res.status).toBe(200);
});

test('unknown route is 404', async () => {
  const app = makeApp();
  const res = await app.handle(new Request('http://localhost/nope'));
  expect(res.status).toBe(404);
});
