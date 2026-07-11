import { get } from 'svelte/store';
import { expect, test } from 'vitest';
import { createThemeStore } from './theme';

test('starts at the given theme and toggles', () => {
  const theme = createThemeStore('light');
  expect(get(theme)).toBe('light');
  theme.toggle();
  expect(get(theme)).toBe('dark');
  theme.toggle();
  expect(get(theme)).toBe('light');
});

test('set overrides the value', () => {
  const theme = createThemeStore('light');
  theme.set('dark');
  expect(get(theme)).toBe('dark');
});
