export interface Poller {
  start(): void;
  stop(): void;
  readonly running: boolean;
}

/**
 * Shared polling primitive so pages don't each re-implement setInterval bookkeeping.
 * Runs `task` immediately on start, then every `intervalMs`, until stopped.
 */
export function createPoller(task: () => void | Promise<void>, intervalMs: number): Poller {
  let handle: ReturnType<typeof setInterval> | null = null;

  return {
    start() {
      if (handle !== null) return;
      void task();
      handle = setInterval(() => void task(), intervalMs);
    },
    stop() {
      if (handle === null) return;
      clearInterval(handle);
      handle = null;
    },
    get running() {
      return handle !== null;
    },
  };
}
