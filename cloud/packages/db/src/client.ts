import { drizzle, type PostgresJsDatabase } from 'drizzle-orm/postgres-js';
import postgres from 'postgres';
import * as schema from './schema';

export type Schema = typeof schema;
export type Db = PostgresJsDatabase<Schema>;

/**
 * Open a Drizzle client over Postgres (postgres.js driver).
 * @param connectionString e.g. `postgres://user:pass@host:5432/db`
 */
export function createDb(connectionString: string): { db: Db; client: postgres.Sql } {
  const client = postgres(connectionString, { max: 10 });
  const db = drizzle({ client, schema });
  return { db, client };
}
