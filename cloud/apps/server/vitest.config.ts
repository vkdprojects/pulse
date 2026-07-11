import { defineConfig } from 'vitest/config';

// Integration tests spin up PGlite (Postgres in WASM) and run migrations per case; give them
// generous timeouts so they don't flake under load when many test files run together.
export default defineConfig({
  test: {
    testTimeout: 30000,
    hookTimeout: 30000,
  },
});
