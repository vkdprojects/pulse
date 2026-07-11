import type { FetchLike } from '@vixpi/contracts';

/**
 * Typed client for a tenant's pulse-master REST API. Only the server holds the admin token;
 * one client is created per org from the org's stored `master_url` + `master_admin_token`.
 */
export interface MasterClientConfig {
  baseUrl: string;
  adminToken: string;
  fetch?: FetchLike;
}

export interface MasterAgent {
  id: string;
  hostname: string;
  status: string;
  version: string;
  cpu_pct: number | null;
  mem_pct: number | null;
  uptime_secs: number | null;
}

export interface MasterOverview {
  total_agents: number;
  online: number;
  offline: number;
  agents: MasterAgent[];
}

export interface ExecResult {
  agent_id: string;
  exit_code: number;
  stdout: string;
  stderr: string;
  timed_out: boolean;
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

  async function send<T>(method: string, path: string, body?: unknown): Promise<T> {
    const headers: Record<string, string> = { authorization: `Bearer ${config.adminToken}` };
    if (body !== undefined) headers['content-type'] = 'application/json';

    const res = await doFetch(`${base}${path}`, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    if (!res.ok) {
      throw new MasterError(res.status, `master ${method} ${path} responded ${res.status}`);
    }
    return (await res.json()) as T;
  }

  return {
    overview: () => send<MasterOverview>('GET', '/api/overview'),
    listAgents: () => send<{ agents: unknown[] }>('GET', '/api/agents'),
    latestMetric: (id: string) => send<unknown>('GET', `/api/agents/${id}/metrics/latest`),
    metrics: (id: string) =>
      send<{ agent_id: string; metrics: unknown[] }>('GET', `/api/agents/${id}/metrics`),
    rename: (id: string, name: string) => send<unknown>('PUT', `/api/agents/${id}/name`, { name }),
    remove: (id: string) => send<unknown>('DELETE', `/api/agents/${id}`),
    exec: (id: string, command: string, timeoutSecs?: number) =>
      send<ExecResult>('POST', `/api/agents/${id}/exec`, {
        command,
        timeout_secs: timeoutSecs ?? 30,
      }),
  };
}

export type MasterClient = ReturnType<typeof createMasterClient>;
