// Loaded only by the better-auth CLI to generate the Drizzle schema (bun run db:auth-generate).
// Not imported by the app. Uses a dummy DB — generation never connects.
import { createDb } from '@vixpi/db';
import { createAuth } from './src/index';

const { db } = createDb('postgres://localhost:5432/generate');

export const auth = createAuth({
  db,
  secret: 'generation-only-secret-not-used-at-runtime',
  baseUrl: 'http://localhost:3000',
  trustedOrigins: ['http://localhost:5173'],
});
