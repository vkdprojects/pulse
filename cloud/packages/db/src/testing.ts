import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { PGlite } from '@electric-sql/pglite';
import { drizzle, type PgliteDatabase } from 'drizzle-orm/pglite';
import * as schema from './schema';

export type TestDb = PgliteDatabase<typeof schema>;

const MIGRATIONS_DIR = new URL('../drizzle', import.meta.url).pathname;

/**
 * Spin up an in-process Postgres (PGlite) with the schema applied from the generated
 * drizzle migrations. Real Postgres semantics, no external service — for tests only.
 * Run `bun run --filter '@vixpi/db' db:generate` first so `drizzle/` exists.
 */
export async function createTestDb(): Promise<{ db: TestDb; client: PGlite }> {
  const client = new PGlite();
  const db = drizzle({ client, schema });

  const files = readdirSync(MIGRATIONS_DIR)
    .filter((f) => f.endsWith('.sql'))
    .sort();

  for (const file of files) {
    const contents = readFileSync(join(MIGRATIONS_DIR, file), 'utf8');
    for (const statement of contents.split('--> statement-breakpoint')) {
      const trimmed = statement.trim();
      if (trimmed.length > 0) {
        await client.exec(trimmed);
      }
    }
  }

  return { db, client };
}
