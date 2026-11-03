import { ApiError, type FleetOverview } from '@vixpi/contracts';
import { writable } from 'svelte/store';
import { api } from '$lib/api';
import { createPoller } from './usePolling';

export interface FleetState {
  loading: boolean;
  error: string | null;
  /** True when the org has no provisioned master yet (API 409). */
  needsMaster: boolean;
  data: FleetOverview | null;
}

/**
 * Shared fleet data source: polls `GET /v1/servers` and exposes a single reactive state.
 * Pages subscribe and call start()/stop() — no page re-implements fetching or polling.
 */
export function createFleet(intervalMs = 5000) {
  const store = writable<FleetState>({
    loading: true,
    error: null,
    needsMaster: false,
    data: null,
  });

  async function refresh() {
    try {
      const data = await api.servers();
      store.set({ loading: false, error: null, needsMaster: false, data });
    } catch (err) {
      if (err instanceof ApiError && err.status === 409) {
        store.set({ loading: false, error: null, needsMaster: true, data: null });
        return;
      }
      store.set({
        loading: false,
        error: err instanceof Error ? err.message : 'Failed to load fleet',
        needsMaster: false,
        data: null,
      });
    }
  }

  const poller = createPoller(refresh, intervalMs);
  return { subscribe: store.subscribe, start: poller.start, stop: poller.stop, refresh };
}
