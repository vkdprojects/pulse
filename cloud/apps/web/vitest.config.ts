import { defineConfig } from 'vitest/config';

// Standalone config (no SvelteKit plugin) so unit tests for stores/hooks/utils run in plain
// Node without compiling Svelte components. Component tests, when added, get their own setup.
export default defineConfig({
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
});
