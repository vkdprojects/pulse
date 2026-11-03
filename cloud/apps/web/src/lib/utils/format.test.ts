import { expect, test } from 'vitest';
import { formatPct, formatUptime } from './format';

test('formatPct rounds and handles null', () => {
  expect(formatPct(23.4)).toBe('23%');
  expect(formatPct(0)).toBe('0%');
  expect(formatPct(null)).toBe('—');
});

test('formatUptime renders compact durations', () => {
  expect(formatUptime(1043000)).toBe('12d 1h');
  expect(formatUptime(3660)).toBe('1h 1m');
  expect(formatUptime(120)).toBe('2m');
  expect(formatUptime(0)).toBe('—');
  expect(formatUptime(null)).toBe('—');
});
