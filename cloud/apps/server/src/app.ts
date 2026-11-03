import { cors } from '@elysiajs/cors';
import { swagger } from '@elysiajs/swagger';
import type { Auth } from '@vixpi/auth';
import { type FetchLike, HealthResponse } from '@vixpi/contracts';
import type { Db } from '@vixpi/db';
import { Elysia } from 'elysia';
import { resolveAuth, resolveOrg } from './context';
import { ingestRoutes } from './routes/ingest';
import { serversRoutes } from './routes/servers';
import { webhooksRoutes } from './routes/webhooks';

export interface AppDeps {
  auth: Auth;
  db: Db;
  corsOrigins: string[];
  version?: string;
  /** Shared secret a master presents to POST /v1/internal/alerts/:orgId. */
  alertIngestSecret?: string;
  /** URL of the install script shown in the add-server command. */
  installScriptUrl?: string;
  /** Injected in tests to stub the per-org pulse-master; defaults to real `fetch`. */
  masterFetch?: FetchLike;
  /** Injected in tests to capture outbound webhook deliveries; defaults to real `fetch`. */
  webhookFetch?: FetchLike;
}

const SERVICE = 'vixpi-pulse-api';

/**
 * Compose the Elysia application. Kept as a factory (no side effects) so tests can build it
 * and drive routes via `app.handle(new Request(...))`.
 */
export function createApp(deps: AppDeps) {
  const version = deps.version ?? '0.0.0';

  return (
    new Elysia()
      .onAfterHandle(({ set }) => {
        set.headers['x-content-type-options'] = 'nosniff';
        set.headers['x-frame-options'] = 'DENY';
        set.headers['referrer-policy'] = 'no-referrer';
      })
      .use(cors({ origin: deps.corsOrigins, credentials: true }))
      .use(
        swagger({
          path: '/docs',
          documentation: {
            info: { title: 'Vixpi Pulse API', version },
            tags: [
              { name: 'meta', description: 'Health & discovery' },
              { name: 'auth', description: 'Authentication & organizations' },
              { name: 'servers', description: 'Fleet & agent management' },
              { name: 'metrics', description: 'Telemetry & time-series' },
              { name: 'alerts', description: 'Alerts & webhooks' },
            ],
          },
        }),
      )
      // better-auth owns everything under /api/auth/* (login, org, members…)
      .all('/api/auth/*', ({ request }) => deps.auth.handler(request))
      .get('/health', () => ({ status: 'ok' as const, service: SERVICE, version }), {
        response: HealthResponse,
        detail: { tags: ['meta'], summary: 'Liveness probe' },
      })
      .group('/v1', (v1) =>
        v1
          .get('/', () => ({ message: 'Vixpi Pulse API v1' }), {
            detail: { tags: ['meta'], summary: 'API v1 root' },
          })
          .get(
            '/me',
            async ({ request, set }) => {
              const ctx = await resolveAuth(deps.auth, request.headers);
              if (!ctx) {
                set.status = 401;
                return { error: { code: 'unauthorized', message: 'Not authenticated' } };
              }
              const org = ctx.activeOrgId
                ? await resolveOrg(deps.db, ctx.user.id, ctx.activeOrgId)
                : null;
              return { user: ctx.user, org };
            },
            { detail: { tags: ['auth'], summary: 'Current user and active organization' } },
          ),
      )
      .use(serversRoutes(deps))
      .use(webhooksRoutes(deps))
      .use(ingestRoutes(deps))
  );
}

export type App = ReturnType<typeof createApp>;
