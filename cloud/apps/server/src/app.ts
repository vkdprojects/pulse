import { cors } from '@elysiajs/cors';
import { swagger } from '@elysiajs/swagger';
import type { Auth } from '@vixpi/auth';
import { HealthResponse } from '@vixpi/contracts';
import { Elysia } from 'elysia';

export interface AppDeps {
  auth: Auth;
  corsOrigins: string[];
  version?: string;
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
      // better-auth owns everything under /api/auth/* (login, org, api-keys…)
      .all('/api/auth/*', ({ request }) => deps.auth.handler(request))
      .get('/health', () => ({ status: 'ok' as const, service: SERVICE, version }), {
        response: HealthResponse,
        detail: { tags: ['meta'], summary: 'Liveness probe' },
      })
      .group('/v1', (app) =>
        app.get('/', () => ({ message: 'Vixpi Pulse API v1' }), {
          detail: { tags: ['meta'], summary: 'API v1 root' },
        }),
      )
  );
}

export type App = ReturnType<typeof createApp>;
