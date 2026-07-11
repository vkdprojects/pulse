import { createAuth } from '@vixpi/auth';
import { createDb } from '@vixpi/db';
import { createApp } from './app';
import { loadEnv } from './env';

const env = loadEnv();
const { db } = createDb(env.DATABASE_URL);

const auth = createAuth({
  db,
  secret: env.BETTER_AUTH_SECRET,
  baseUrl: env.BETTER_AUTH_URL,
  trustedOrigins: env.corsOrigins,
});

const app = createApp({
  auth,
  db,
  corsOrigins: env.corsOrigins,
  alertIngestSecret: env.ALERT_INGEST_SECRET || undefined,
  version: process.env.PULSE_VERSION ?? '0.1.0',
});

app.listen(env.PORT);

console.info(`vixpi-pulse-api listening on :${env.PORT} (Swagger at /docs)`);
