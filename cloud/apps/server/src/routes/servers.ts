import { auditLog } from '@vixpi/db';
import { Elysia, t } from 'elysia';
import type { AppDeps } from '../app';
import { isContextError, resolveServerContext, type ServerContextError } from '../context';

function fail(set: { status?: number | string }, error: ServerContextError) {
  set.status = error.status;
  return { error: { code: error.code, message: error.message } };
}

/**
 * Org-scoped fleet management — every route proxies to the caller's org master (admin token
 * stays server-side). Guarded by `resolveServerContext` (auth → active org → membership →
 * provisioned master). Mounted under /v1/servers.
 */
export function serversRoutes(deps: AppDeps) {
  const guard = (request: Request) => resolveServerContext(deps, request.headers, deps.masterFetch);

  return new Elysia({ prefix: '/v1/servers' })
    .get(
      '/',
      async ({ request, set }) => {
        const ctx = await guard(request);
        if (isContextError(ctx)) return fail(set, ctx.error);
        const raw = await ctx.master.overview();
        // Normalize the master's snake_case into the product's camelCase contract.
        return {
          totalAgents: raw.total_agents,
          online: raw.online,
          offline: raw.offline,
          agents: raw.agents.map((a) => ({
            id: a.id,
            hostname: a.hostname,
            status: a.status ?? 'offline',
            version: a.version ?? '',
            cpuPct: a.cpu_pct ?? null,
            memPct: a.mem_pct ?? null,
            uptimeSecs: a.uptime_secs ?? null,
          })),
        };
      },
      { detail: { tags: ['servers'], summary: 'Fleet overview' } },
    )
    .get(
      '/:id',
      async ({ request, params, set }) => {
        const ctx = await guard(request);
        if (isContextError(ctx)) return fail(set, ctx.error);
        return await ctx.master.latestMetric(params.id);
      },
      { detail: { tags: ['servers'], summary: 'Latest metrics snapshot for an agent' } },
    )
    .get(
      '/:id/metrics',
      async ({ request, params, set }) => {
        const ctx = await guard(request);
        if (isContextError(ctx)) return fail(set, ctx.error);
        return await ctx.master.metrics(params.id);
      },
      { detail: { tags: ['metrics'], summary: 'Recent metrics for an agent' } },
    )
    .put(
      '/:id/name',
      async ({ request, params, body, set }) => {
        const ctx = await guard(request);
        if (isContextError(ctx)) return fail(set, ctx.error);
        if (ctx.org.role === 'viewer') {
          return fail(set, { status: 403, code: 'forbidden', message: 'Viewers are read-only' });
        }
        return await ctx.master.rename(params.id, body.name);
      },
      {
        body: t.Object({ name: t.String({ minLength: 1, maxLength: 200 }) }),
        detail: { tags: ['servers'], summary: 'Rename an agent' },
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
        return await ctx.master.remove(params.id);
      },
      { detail: { tags: ['servers'], summary: 'Remove an agent' } },
    )
    .post(
      '/:id/exec',
      async ({ request, params, body, set }) => {
        const ctx = await guard(request);
        if (isContextError(ctx)) return fail(set, ctx.error);
        if (ctx.org.role === 'viewer') {
          return fail(set, { status: 403, code: 'forbidden', message: 'Viewers cannot exec' });
        }

        const result = await ctx.master.exec(params.id, body.command, body.timeoutSecs);

        // High-risk surface — audit who ran what, where (design §14).
        await deps.db.insert(auditLog).values({
          id: crypto.randomUUID(),
          orgId: ctx.org.id,
          actorId: ctx.user.id,
          action: 'server.exec',
          targetType: 'agent',
          targetId: params.id,
          metadata: { command: body.command },
        });

        return {
          agentId: result.agent_id,
          exitCode: result.exit_code,
          stdout: result.stdout,
          stderr: result.stderr,
          timedOut: result.timed_out,
        };
      },
      {
        body: t.Object({
          command: t.String({ minLength: 1 }),
          timeoutSecs: t.Optional(t.Integer({ minimum: 1, maximum: 300 })),
        }),
        detail: { tags: ['servers'], summary: 'Execute a command on an agent (audited)' },
      },
    );
}
