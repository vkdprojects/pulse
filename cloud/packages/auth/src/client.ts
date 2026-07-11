import { adminClient, organizationClient } from 'better-auth/client/plugins';
import { createAuthClient } from 'better-auth/svelte';
import { ac, roles } from './permissions';

/**
 * Browser-side auth client for the SvelteKit app. Plugin clients mirror the server plugins
 * (same access-control roles) so the UI gets typed org / admin methods. No secrets — talks
 * to the API only.
 */
export function createVixpiAuthClient(baseURL: string) {
  return createAuthClient({
    baseURL,
    plugins: [organizationClient({ ac, roles }), adminClient()],
  });
}

export type VixpiAuthClient = ReturnType<typeof createVixpiAuthClient>;
