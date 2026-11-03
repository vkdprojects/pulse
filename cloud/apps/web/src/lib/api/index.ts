import { createApiClient } from '@vixpi/contracts';
import { env } from '$env/dynamic/public';

/**
 * The single typed entry point to the API for the whole web app (AGENTS.md §4).
 * Import `api` — never call `fetch` against the backend directly.
 */
export const api = createApiClient({
  baseUrl: env.PUBLIC_API_URL ?? 'http://localhost:3000',
});
