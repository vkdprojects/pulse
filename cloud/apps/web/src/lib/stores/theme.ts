import { writable } from 'svelte/store';

export type Theme = 'light' | 'dark';

/**
 * App theme as a Svelte store. Kept framework-light (svelte/store) so it can be unit-tested
 * and shared across components without duplicating the toggle logic.
 */
export function createThemeStore(initial: Theme = 'light') {
  const store = writable<Theme>(initial);
  return {
    subscribe: store.subscribe,
    set: store.set,
    toggle: () => store.update((t) => (t === 'light' ? 'dark' : 'light')),
  };
}

export const theme = createThemeStore();
