import { Value } from '@sinclair/typebox/value';
import { expect, test } from 'vitest';
import { ApiError, createApiClient, type FetchLike } from './client';
import { CreateOrganizationBody, HealthResponse } from './schemas';

test('HealthResponse validates good and bad payloads', () => {
  expect(Value.Check(HealthResponse, { status: 'ok', service: 'api', version: '0.1.0' })).toBe(
    true,
  );
  expect(Value.Check(HealthResponse, { status: 'down', service: 'api', version: '0.1.0' })).toBe(
    false,
  );
  expect(Value.Check(HealthResponse, { status: 'ok' })).toBe(false);
});

test('CreateOrganizationBody enforces slug pattern', () => {
  expect(Value.Check(CreateOrganizationBody, { name: 'Acme', slug: 'acme-1' })).toBe(true);
  expect(Value.Check(CreateOrganizationBody, { name: 'Acme', slug: 'Acme Inc' })).toBe(false);
});

test('client returns a validated health response', async () => {
  const fetchMock: FetchLike = async () =>
    new Response(JSON.stringify({ status: 'ok', service: 'api', version: '0.1.0' }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    });

  const client = createApiClient({ baseUrl: 'http://x', fetch: fetchMock });
  const health = await client.health();
  expect(health.service).toBe('api');
});

test('client throws a typed ApiError on error envelope', async () => {
  const fetchMock: FetchLike = async () =>
    new Response(JSON.stringify({ error: { code: 'unauthorized', message: 'nope' } }), {
      status: 401,
      headers: { 'content-type': 'application/json' },
    });

  const client = createApiClient({ baseUrl: 'http://x', fetch: fetchMock });
  await expect(client.health()).rejects.toMatchObject({ status: 401, code: 'unauthorized' });
  await expect(client.health()).rejects.toBeInstanceOf(ApiError);
});
