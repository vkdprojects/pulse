import { type Static, Type } from '@sinclair/typebox';
import { Value } from '@sinclair/typebox/value';

const EnvSchema = Type.Object({
  DATABASE_URL: Type.String({ minLength: 1 }),
  BETTER_AUTH_SECRET: Type.String({ minLength: 16 }),
  BETTER_AUTH_URL: Type.String({ minLength: 1 }),
  /** Comma-separated allow-list of browser origins. */
  CORS_ORIGINS: Type.String({ default: 'http://localhost:5173' }),
  /** Shared secret masters present when POSTing alerts to the ingest endpoint. */
  ALERT_INGEST_SECRET: Type.String({ default: '' }),
  PORT: Type.Integer({ minimum: 1, maximum: 65535, default: 3000 }),
});

export type Env = Static<typeof EnvSchema> & { corsOrigins: string[] };

/**
 * Validate process env at startup. Fails fast with a readable message rather than letting a
 * missing secret surface as a runtime 500 later.
 */
export function loadEnv(source: Record<string, string | undefined> = process.env): Env {
  const candidate = Value.Convert(EnvSchema, Value.Default(EnvSchema, { ...source }));

  if (!Value.Check(EnvSchema, candidate)) {
    const issues = [...Value.Errors(EnvSchema, candidate)]
      .map((e) => `${e.path || '/'}: ${e.message}`)
      .join('; ');
    throw new Error(`Invalid environment: ${issues}`);
  }

  return {
    ...candidate,
    corsOrigins: candidate.CORS_ORIGINS.split(',')
      .map((o) => o.trim())
      .filter(Boolean),
  };
}
