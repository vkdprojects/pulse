import type { FetchLike } from '@vixpi/contracts';
import { expect, test } from 'vitest';
import { createMasterClient, MasterError } from './master';

test('overview injects the bearer token and returns parsed data', async () => {
  let seenAuth: string | null = null;
  let seenUrl = '';
  const fetchMock: FetchLike = async (input, init) => {
    seenUrl = String(input);
    seenAuth = new Headers(init?.headers).get('authorization');
    return new Response(
      JSON.stringify({ total_agents: 2, online: 1, offline: 1, agents: [{}, {}] }),
      { status: 200, headers: { 'content-type': 'application/json' } },
    );
  };

  const master = createMasterClient({
    baseUrl: 'http://master:8080/',
    adminToken: 'tok',
    fetch: fetchMock,
  });
  const overview = await master.overview();

  expect(seenUrl).toBe('http://master:8080/api/overview');
  expect(seenAuth).toBe('Bearer tok');
  expect(overview.total_agents).toBe(2);
});

test('non-2xx throws a typed MasterError', async () => {
  const fetchMock: FetchLike = async () => new Response('nope', { status: 502 });
  const master = createMasterClient({ baseUrl: 'http://m', adminToken: 't', fetch: fetchMock });

  await expect(master.overview()).rejects.toBeInstanceOf(MasterError);
  await expect(master.overview()).rejects.toMatchObject({ status: 502 });
});
