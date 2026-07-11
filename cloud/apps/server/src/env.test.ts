import { expect, test } from 'vitest';
import { loadEnv } from './env';

const base = {
  DATABASE_URL: 'postgres://pulse:pulse@localhost:5432/pulse',
  BETTER_AUTH_SECRET: 'a-sufficiently-long-secret',
  BETTER_AUTH_URL: 'http://localhost:3000',
};

test('loads defaults and splits CORS origins', () => {
  const env = loadEnv({ ...base, CORS_ORIGINS: 'http://a.com, http://b.com' });
  expect(env.PORT).toBe(3000);
  expect(env.corsOrigins).toEqual(['http://a.com', 'http://b.com']);
});

test('coerces PORT to an integer', () => {
  const env = loadEnv({ ...base, PORT: '8080' });
  expect(env.PORT).toBe(8080);
});

test('rejects a too-short auth secret', () => {
  expect(() => loadEnv({ ...base, BETTER_AUTH_SECRET: 'short' })).toThrow(/Invalid environment/);
});

test('rejects a missing database url', () => {
  const { DATABASE_URL: _omit, ...withoutDb } = base;
  expect(() => loadEnv(withoutDb)).toThrow(/Invalid environment/);
});
