/**
 * Typed client for a tenant's pulse-master REST API. Only the server holds the admin token;
 * one client is created per org from the org's stored `master_url` + `master_admin_token`.
 */
import type { FetchLike } from '@vixpi/contracts';

export interface MasterClientConfig {
  baseUrl: string;
  adminToken: string;
  fetch?: FetchLike;
}

export interface MasterOverview {
  total_agents: number;
  online: number;
  offline: number;
  agents: unknown[];
}

export class MasterError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = 'MasterError';
  }
}

export function createMasterClient(config: MasterClientConfig) {
  const doFetch: FetchLike = config.fetch ?? fetch;
  const base = config.baseUrl.replace(/\/+$/, '');

  async function get<T>(path: string): Promise<T> {
    const res = await doFetch(`${base}${path}`, {
      headers: { authorization: `Bearer ${config.adminToken}` },
    });
    if (!res.ok) {
      throw new MasterError(res.status, `master ${path} responded ${res.status}`);
    }
    return (await res.json()) as T;
  }

  return {
    overview: () => get<MasterOverview>('/api/overview'),
    listAgents: () => get<{ agents: unknown[] }>('/api/agents'),
  };
}

export type MasterClient = ReturnType<typeof createMasterClient>;
