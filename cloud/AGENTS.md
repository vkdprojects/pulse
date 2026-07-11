# AGENTS.md — `cloud/` (Vixpi Pulse SaaS)

Rules for any human or AI contributor working inside `cloud/`. The Rust workspace at the repo
root is a **separate world** — never touch `crates/`, `migrations/`, or root `Cargo.*` from
here. Design rationale lives in `docs/admin-panel-design.md`.

## 1. What this is

The TypeScript side of Vixpi Pulse: a hosted, multi-tenant SaaS control plane on top of the
Rust `pulse-master` telemetry engine. One Bun workspace, split into runnable `apps/*` and
shared `packages/*`.

## 2. Tech stack (keep it current, no surprises)

- **Runtime:** Bun (>= 1.3). **Language:** TypeScript 7, `strict` everywhere.
- **API:** Elysia (`apps/server`) + `@elysiajs/swagger` + `@elysiajs/cors`.
- **Web:** SvelteKit + Svelte 5 (runes) + Bits UI + Tailwind CSS 4 + uPlot (`apps/web`).
- **DB:** Drizzle ORM over Postgres (postgres.js) in `packages/db`; PGlite for tests.
- **TypeScript:** 7 for the API and packages; `apps/web` pins TS 5.9 (svelte-check isn't TS7-ready yet).
- **Auth:** better-auth — `organization` / `admin` / `apiKey` plugins (`packages/auth`).
- **Contracts:** shared types + typed client (`packages/contracts`).
- **Tooling:** Biome (lint + format), Vitest (tests).

Add a dependency only with a reason; prefer the latest stable. Never pin to abandoned libs.

## 3. Package boundaries (respect the arrows)

```
apps/web ─┐
          ├─▶ packages/contracts ─▶ (types only)
apps/server ─▶ packages/{db, auth, contracts}
packages/auth ─▶ packages/db
```

- `apps/*` may depend on `packages/*`, never on another app.
- `packages/*` never import from `apps/*`.
- Only `apps/server` talks to `pulse-master`. The browser never holds the admin token.
- Internal packages are consumed as source (`exports` → `./src/index.ts`); no build step.

## 4. Non-negotiable conventions

- **No god files.** One responsibility per file/component. Soft cap **~200 lines**; split
  before you exceed it. A route composes components — it does not inline everything.
- **Strong typing, no escape hatches.** No `any`, no `as any`, no `@ts-ignore`. Use `unknown`
  + validation at boundaries. `satisfies` over casts. Every exported symbol has an explicit
  type at its boundary.
- **Validate all input.** Every Elysia route declares a TypeBox schema (`body`/`query`/
  `params`); nothing untrusted reaches a handler unvalidated.
- **No duplication.** Cross-cutting logic lives in a package or a shared hook/store, not
  copy-pasted. In `apps/web`, shared behavior goes in `src/lib/hooks/` and `src/lib/stores/`
  (e.g. `useAgents`, `usePolling`, `useAuth`), UI primitives in `src/lib/components/ui/`.
- **One way to call the backend.** The web app talks to the API only through the typed client
  from `packages/contracts`. No scattered `fetch`.
- **Secrets stay server-side.** Never import server-only modules (`packages/db`, `auth`,
  master client, env secrets) into browser code.
- **Errors:** typed error envelope from the API; never leak stack traces or secrets to clients.
- **Multi-tenancy:** every DB query and master call is scoped by `org_id`, enforced in
  middleware — never trust an `org_id` from the client body.

## 5. Tests are part of the work

- Vitest colocated as `*.test.ts` next to the code (or `*.spec.ts`).
- Every package/app ships tests; new behavior lands with a test that asserts **values**, not
  just "doesn't throw".
- API routes tested via `app.handle(new Request(...))`. Stores/hooks/utils unit-tested in
  isolation. Mock the network; never hit a real `pulse-master` in unit tests.

## 6. Before you commit

From `cloud/`:

```bash
bun run check    # biome + tsc --noEmit + vitest, across every workspace
```

Green `check` is the minimum bar. Keep commits focused; follow the repo's
`type: description` convention (`feat`, `fix`, `test`, `docs`, `chore`).

## 7. Layout

```
apps/server      Elysia API (the product surface)
apps/web         SvelteKit UI (thin client, no secrets)
packages/db      Drizzle schema + migrations + client
packages/auth    better-auth config (server + client)
packages/contracts  shared types + typed SDK
```
