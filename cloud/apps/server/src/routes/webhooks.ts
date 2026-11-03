import { randomBytes } from 'node:crypto';
import { type WebhookSubscription, webhookSubscriptions } from '@vixpi/db';
import { and, desc, eq } from 'drizzle-orm';
import { Elysia, t } from 'elysia';
import type { AppDeps } from '../app';
import { isContextError, resolveOrgContext, type ServerContextError } from '../context';
import { deliver, recentDeliveries } from '../webhooks/dispatch';

function fail(set: { status?: number | string }, error: ServerContextError) {
  set.status = error.status;
  return { error: { code: error.code, message: error.message } };
}

/** Public shape of a subscription — never exposes the signing secret. */
function toDto(sub: WebhookSubscription) {
  return {
    id: sub.id,
    scope: sub.scope,
    target: sub.target,
    url: sub.url,
    events: sub.events,
    active: sub.active,
    createdAt: sub.createdAt.toISOString(),
  };
}

/**
 * Per-org webhook management (panel-side, no master needed). Create returns the signing
 * secret once; it is never returned again. Mutations require admin/owner (viewers read-only).
 * Mounted under /v1/webhooks.
 */
export function webhooksRoutes(deps: AppDeps) {
  const guard = (request: Request) => resolveOrgContext(deps, request.headers);

  return new Elysia({ prefix: '/v1/webhooks' })
    .get(
      '/',
      async ({ request, set }) => {
        const ctx = await guard(request);
        if (isContextError(ctx)) return fail(set, ctx.error);
        const subs = await deps.db
          .select()
          .from(webhookSubscriptions)
          .where(eq(webhookSubscriptions.orgId, ctx.org.id))
          .orderBy(desc(webhookSubscriptions.createdAt));
        return { webhooks: subs.map(toDto) };
      },
      { detail: { tags: ['alerts'], summary: 'List webhook subscriptions' } },
    )
    .post(
      '/',
      async ({ request, body, set }) => {
        const ctx = await guard(request);
        if (isContextError(ctx)) return fail(set, ctx.error);
        if (ctx.org.role === 'viewer') {
          return fail(set, { status: 403, code: 'forbidden', message: 'Viewers are read-only' });
        }
        if (!/^https?:\/\//.test(body.url)) {
          return fail(set, { status: 400, code: 'invalid_url', message: 'url must be http(s)' });
        }

        const id = crypto.randomUUID();
        const secret = randomBytes(24).toString('hex');
        const [created] = await deps.db
          .insert(webhookSubscriptions)
          .values({
            id,
            orgId: ctx.org.id,
            scope: body.scope,
            target: body.target ?? null,
            url: body.url,
            secret,
            events: body.events,
            createdBy: ctx.user.id,
          })
          .returning();

        // Secret is shown exactly once, at creation.
        return { ...(created ? toDto(created) : { id }), secret };
      },
      {
        body: t.Object({
          scope: t.Union([t.Literal('org'), t.Literal('server'), t.Literal('user')]),
          target: t.Optional(t.String()),
          url: t.String({ minLength: 1 }),
          events: t.Array(t.String({ minLength: 1 }), { minItems: 1 }),
        }),
        detail: { tags: ['alerts'], summary: 'Create a webhook subscription' },
      },
    )
    .delete(
      '/:id',
      async ({ request, params, set }) => {
        const ctx = await guard(request);
        if (isContextError(ctx)) return fail(set, ctx.error);
        if (ctx.org.role === 'viewer') {
          return fail(set, { status: 403, code: 'forbidden', message: 'Viewers are read-only' });
        }
        const deleted = await deps.db
          .delete(webhookSubscriptions)
          .where(
            and(eq(webhookSubscriptions.id, params.id), eq(webhookSubscriptions.orgId, ctx.org.id)),
          )
          .returning();
        if (deleted.length === 0) {
          return fail(set, { status: 404, code: 'not_found', message: 'Subscription not found' });
        }
        return { deleted: true };
      },
      { detail: { tags: ['alerts'], summary: 'Delete a webhook subscription' } },
    )
    .post(
      '/:id/test',
      async ({ request, params, set }) => {
        const ctx = await guard(request);
        if (isContextError(ctx)) return fail(set, ctx.error);
        if (ctx.org.role === 'viewer') {
          return fail(set, { status: 403, code: 'forbidden', message: 'Viewers are read-only' });
        }
        const [sub] = await deps.db
          .select()
          .from(webhookSubscriptions)
          .where(
            and(eq(webhookSubscriptions.id, params.id), eq(webhookSubscriptions.orgId, ctx.org.id)),
          )
          .limit(1);
        if (!sub) {
          return fail(set, { status: 404, code: 'not_found', message: 'Subscription not found' });
        }
        const outcome = await deliver(
          deps.db,
          sub,
          { event: 'test.ping', data: { message: 'Vixpi Pulse test event' } },
          deps.webhookFetch,
        );
        return outcome;
      },
      { detail: { tags: ['alerts'], summary: 'Send a test event to a subscription' } },
    )
    .get(
      '/:id/deliveries',
      async ({ request, params, set }) => {
        const ctx = await guard(request);
        if (isContextError(ctx)) return fail(set, ctx.error);
        const [sub] = await deps.db
          .select()
          .from(webhookSubscriptions)
          .where(
            and(eq(webhookSubscriptions.id, params.id), eq(webhookSubscriptions.orgId, ctx.org.id)),
          )
          .limit(1);
        if (!sub) {
          return fail(set, { status: 404, code: 'not_found', message: 'Subscription not found' });
        }
        const deliveries = await recentDeliveries(deps.db, sub.id);
        return {
          deliveries: deliveries.map((d) => ({
            id: d.id,
            event: d.event,
            status: d.status,
            responseStatus: d.responseStatus,
            error: d.error,
            createdAt: d.createdAt.toISOString(),
          })),
        };
      },
      { detail: { tags: ['alerts'], summary: 'Recent deliveries for a subscription' } },
    );
}
