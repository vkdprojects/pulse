import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { createPoller } from './usePolling';

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

test('runs immediately, then on each interval, and stops cleanly', () => {
  let count = 0;
  const poller = createPoller(() => {
    count += 1;
  }, 1000);

  poller.start();
  expect(count).toBe(1);
  expect(poller.running).toBe(true);

  vi.advanceTimersByTime(3000);
  expect(count).toBe(4);

  poller.stop();
  vi.advanceTimersByTime(5000);
  expect(count).toBe(4);
  expect(poller.running).toBe(false);
});

test('double start does not schedule twice', () => {
  let count = 0;
  const poller = createPoller(() => {
    count += 1;
  }, 1000);

  poller.start();
  poller.start();
  vi.advanceTimersByTime(1000);
  // one immediate + one interval tick = 2 (not 3)
  expect(count).toBe(2);
  poller.stop();
});
