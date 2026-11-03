import { timingSafeEqual } from 'node:crypto';
import { Elysia, t } from 'elysia';
import type { AppDeps } from '../app';
import { dispatch } from '../webhooks/dispatch';

function secretsMatch(a: string, b: string): boolean {
  const ba = Buffer.from(a);
  const bb = Buffer.from(b);
  return ba.length === bb.length && timingSafeEqual(ba, bb);
}

/**
 * Internal alert ingest. Each org's pulse-master posts threshold breaches here (its
 * `PULSE_ALERT_WEBHOOK` is set to `/v1/internal/alerts/<orgId>` during provisioning). Guarded
 * by a shared secret; fans the event out to the org's matching subscriptions. See design §8.
 */
export function ingestRoutes(deps: AppDeps) {
  return new Elysia({ prefix: '/v1/internal' }).post(
    '/alerts/:orgId',
    async ({ request, params, body, set }) => {
      if (!deps.alertIngestSecret) {
        set.status = 503;
        return { error: { code: 'ingest_disabled', message: 'Alert ingest is not configured' } };
      }
      const provided = request.headers.get('x-ingest-secret') ?? '';
      if (!secretsMatch(provided, deps.alertIngestSecret)) {
        set.status = 401;
        return { error: { code: 'unauthorized', message: 'Invalid ingest secret' } };
      }

      const outcomes = await dispatch(
        deps.db,
        params.orgId,
        { event: body.event, agentId: body.agentId, userId: body.userId, data: body.data },
        deps.webhookFetch,
      );
      return { dispatched: outcomes.length, outcomes };
    },
    {
      body: t.Object({
        event: t.String({ minLength: 1 }),
        agentId: t.Optional(t.String()),
        userId: t.Optional(t.String()),
        data: t.Optional(t.Record(t.String(), t.Unknown())),
      }),
      detail: { tags: ['alerts'], summary: 'Ingest an alert from a master and fan out' },
    },
  );
}
