# Admin Panel & Service API — Design Document

> Status: **M0 scaffolded.** The `cloud/` workspace exists and is green (lint + typecheck +
> tests). This document captures the agreed architecture; milestones (§16) track progress.

## 1. Motivation

Vixpi Pulse today has three faces: the `pulse` TUI (operators), pre-built Grafana
dashboards (visualization), and the `pulse-master` REST API (the machine interface).
There is **no multi-user web control plane**, and no productized, documented service API.

Two goals now drive this work:

1. A web **admin panel** — multi-user control plane to manage servers, view metrics, onboard
   agents, run commands, and view/configure alerts from a browser.
2. A sellable **service** — Pulse will be offered as a **hosted, multi-tenant SaaS**
   (decided). That means a documented public **API** (Swagger), real authentication,
   input validation, security middleware, per-customer webhooks, and org-based
   multi-tenancy from day one. See §4.

### Explicit non-goals for v1

- Billing/metering integration (Stripe etc.) — designed around, not built yet.
- SSH-based remote provisioning from the browser (phase 2 — see §10).
- Re-implementing every Grafana dashboard on day one (that is the §11 roadmap, not v1).

## 2. Core decision: keep user/tenant logic OUT of Rust

`pulse-master` is a lean telemetry engine (5.8 MB, zero external deps, zero `unsafe`).
Adding accounts, sessions, RBAC, tenancy and webhooks to it would betray that design.

Instead, all of that lives in a TypeScript layer under a single `cloud/` Bun workspace (§12),
split into two apps + a shared package:

- **`cloud/apps/server` — Elysia (Bun):** the product API and gateway. Owns input
  validation, Swagger docs, security middleware, webhook subscriptions + dispatch, and the
  only client that talks to `pulse-master` (holding its admin token). The sellable, documented
  surface.
- **`cloud/apps/web` — SvelteKit (Bun):** the web UI. Consumes the API; holds no secrets.
- **`cloud/packages/{db,auth,contracts}`:** shared database (Drizzle schema/migrations/client),
  better-auth config, and request/response types + typed SDK — so the two apps never duplicate
  or drift.

```
  Browser (SvelteKit UI)
        │  session cookie
        ▼
  ┌───────────────────────── Elysia API (Bun) ─────────────────────────┐
  │  better-auth (users, sessions, orgs, API keys)                     │
  │  input validation (TypeBox)   ·   Swagger / OpenAPI                 │
  │  security middleware (CORS, rate-limit, headers)                    │
  │  webhook subscriptions + dispatch (per org / server / user)        │
  └───────────────┬───────────────────────────────┬────────────────────┘
                  │ Bearer PULSE_ADMIN_TOKEN       │
                  ▼                                ▼
          pulse-master REST :8080          Postgres + Drizzle
        (per tenant — see §4)          (auth, orgs, webhooks, audit)
```

Consequences:

- The `PULSE_ADMIN_TOKEN` **never reaches the browser.** It lives only in the Elysia
  process, which calls the master machine-to-machine.
- The user/tenant layer lives in TypeScript, where it is cheap to evolve.
- **The Rust master stays unchanged** except a few small, surgical read endpoints (§9) and
  pointing its existing alert webhook at the API (§8). No user/tenant logic enters Rust.

### Why not Clerk / an external auth SaaS

Vixpi Pulse's premise is "runs inside your server, zero external dependencies." We use
**better-auth** — a self-hosted, framework-agnostic TypeScript auth library — instead of a
SaaS. All auth data stays in our own database. Deliberate rejection of Clerk/Auth0/etc.

## 3. Tech stack

| Concern       | Choice                          | Rationale                                                  |
|---------------|---------------------------------|------------------------------------------------------------|
| Runtime       | **Bun**                         | Fast, single toolchain for both apps                       |
| API framework | **Elysia**                      | Bun-native; end-to-end typed; TypeBox validation + Swagger built into the route definitions |
| Validation    | **TypeBox** (Elysia `t`)        | One schema drives runtime validation, TS types, and OpenAPI |
| API docs      | **@elysiajs/swagger**           | OpenAPI/Swagger generated from the route schemas — always in sync |
| Web UI        | **SvelteKit** (adapter-node/bun)| Client for the API; fast, minimal                          |
| UI components | **Bits UI**                     | Headless/accessible Svelte primitives — behavior without imposed styling |
| Styling       | **Tailwind CSS**                | Utility-first; 100% responsive, modern, consistent design tokens |
| Lint/format   | **BiomeJS**                     | Single fast tool, no ESLint+Prettier sprawl                |
| Auth          | **better-auth**                 | Self-hosted; email/password, sessions, plus `organization`, `admin`, `apiKey` plugins for tenancy/RBAC/programmatic access |
| App DB        | **Postgres** (Drizzle, postgres.js) | Backs better-auth + orgs/webhooks/audit; same engine family as the master's TimescaleDB. PGlite for tests |
| Charts        | **uPlot**                       | ~40 KB, built for time series; matches the lean ethos      |

Dependency discipline mirrors the Rust side: add a dependency only with justification.

## 4. Multi-tenancy (decided: hosted multi-tenant SaaS)

Pulse is offered as a **hosted, multi-tenant SaaS**. We run the API; customers sign up, each
in their own **organization**. An org's servers, metrics, alerts and webhooks are isolated
from every other org.

- **Isolation model:** since `pulse-master` is single-tenant, each org gets **its own master**.
  The `organizations` table stores that org's `master_url` + `master_admin_token`; the API
  resolves the caller's org on every request and routes to that org's master. This gives hard
  data isolation while keeping the master unchanged.
- **Scoping:** every app-DB query is scoped by `org_id`. A user's requests can only ever
  reach their own org's master — enforced in a middleware layer, not per-handler.
- **Foundation:** better-auth's **`organization` plugin** models users ↔ orgs ↔ roles.
  Sign-up creates a new org with the signer as **owner** (§5).
- **Org → master provisioning** (how a new org gets a master) is an operational open question
  — see §15. v1 can start with manual/assisted master provisioning per org while the
  self-service flow is designed.

Built org-scoped from day one so no retrofit is ever needed.

## 5. Authentication & authorization

Auth is handled by **better-auth** (config in `packages/auth`, schema in `packages/db`),
mounted inside the Elysia API (`/api/auth/*`). The SvelteKit UI uses the better-auth client against that
same origin.

- **Providers:** email & password to start. Social/SSO later via better-auth providers if
  customers ask.
- **Registration:** public sign-up **creates a new organization** with the signer as its
  **owner**. Additional members join by invite (owner/admin invites). Managed via the
  better-auth `organization` plugin.
- **Passwords:** hashed by better-auth (scrypt default). Never logged or returned.
- **Sessions:** DB-backed, delivered as httpOnly, SameSite=Lax, Secure cookies; validated on
  every request via better-auth (`getSession`) populating request context / `event.locals`.
- **Roles / RBAC (launch set):** **owner / admin / viewer**, per org, via the better-auth
  `organization` plugin. Viewer is read-only; admin operates the fleet (rename/remove/exec,
  webhooks); owner also manages members, billing and org settings.
- **Programmatic API access (for selling):** better-auth **`apiKey` plugin** issues per-org
  API keys so customers can call the documented API from their own tooling/CI, rate-limited
  and scoped per key.
- **Master authorization:** the API attaches the resolved org's `PULSE_ADMIN_TOKEN` to
  outbound master calls. End users and API keys never see it.

## 6. API layer: validation, docs, security

The Elysia API is the product surface, so it is specified, not ad-hoc:

- **Input validation:** every route declares a TypeBox schema (`body`, `query`, `params`);
  Elysia validates and rejects bad input before handlers run, and infers types end-to-end.
- **Swagger/OpenAPI:** `@elysiajs/swagger` generates a complete, **tag-organized** doc
  straight from those schemas (grouped e.g. `auth`, `servers`, `metrics`, `alerts`,
  `webhooks`), served at `/docs`. Because docs derive from the schemas, they never drift.
- **Security middleware (applied globally):**
  - CORS allow-list (`@elysiajs/cors`).
  - Rate limiting per IP and per API key (Elysia rate-limit plugin + better-auth key limits).
  - Security headers (CSP, HSTS, X-Content-Type-Options, etc.).
  - Consistent typed error envelope; no stack traces or secrets leaked to clients.
  - Request/audit logging (who/what/when) for state-changing and `exec` routes.
- **Versioning:** routes under `/v1` from the start so the public contract can evolve.

## 7. Web UI (SvelteKit + Bits UI + Tailwind)

Thin client over the API: login, fleet overview, agent detail (live + range charts),
rename/remove/exec, alerts, webhook management, add-server. Holds no secrets — all data and
mutations go through the API with the session cookie.

**Stack:** SvelteKit for routing/data-loading, **Bits UI** for accessible headless
primitives (dialog, dropdown, tabs, tooltip…), **Tailwind** for styling and full
responsiveness, **uPlot** for time-series charts.

**DX & structure conventions (enforced from M0):**

- **No god files.** Single responsibility per component; a route's `+page.svelte` composes
  small components, it does not inline everything. Soft cap ~150–200 lines per component.
- **Component library:** `src/lib/components/ui/` wraps Bits UI primitives with our Tailwind
  design tokens (Button, Card, Dialog, Table, StatTile…); feature components live under
  `src/lib/components/<feature>/`. UI primitives never call the API directly.
- **Global hooks/stores to avoid duplication:** shared logic lives in `src/lib/hooks/` and
  `src/lib/stores/` — e.g. `useAgents`, `usePolling`, `useAuth`, `useToast` — instead of
  being re-implemented per page. One source of truth for fetching, polling, and auth state.
- **Typed API client:** the UI talks to the backend only through the shared client from
  `cloud/packages/contracts` (re-exported via `src/lib/api/`), derived from the Elysia
  types / OpenAPI. No scattered `fetch`, and the same types back both apps.
- **Design tokens:** colors/spacing/typography as Tailwind theme tokens; dark mode via
  Tailwind; responsive by default (mobile-first utilities), verified at common breakpoints.
- **Formatting/linting:** Biome; components and stores are unit-testable in isolation.

## 8. Webhooks for alerts (per org / server / user)

Requirement: customers configure their own alert webhooks, scoped **per server and/or per
user (within their org)**.

- **Ingest:** `pulse-master`'s existing alert engine already fires an outbound webhook on
  threshold breaches. We point its `PULSE_ALERT_WEBHOOK` at a secured API ingest endpoint
  (`POST /v1/internal/alerts`). **No Rust change** beyond configuration.
- **Subscriptions:** stored in the app DB — `webhook_subscriptions(id, org_id, scope, target,
  url, secret, events, active, created_by)` where `scope ∈ {org, server, user}`.
- **Dispatch:** on ingest, the API matches the alert's org + agent to subscriptions and
  fans out with delivery guarantees: **HMAC-signed** payloads (per-subscription secret),
  retries with backoff, and a `webhook_deliveries` log (status, attempts, response) for a
  UI "recent deliveries" view.
- **Management:** CRUD via the API + UI; "send test event" to validate an endpoint.

## 9. Required Rust additions (minimal, surgical)

Approved: small read-only additions to the master API. No user/tenant logic in Rust.

1. **Time-range metrics endpoint** — the current API only returns the last ~20 readings.
   Charts and Grafana replacement (§11) need windowed, downsampled series:

   ```
   GET /api/agents/:id/metrics?from=<ts>&to=<ts>&step=<seconds>&fields=cpu,mem,...
   ```

   Backed by TimescaleDB `time_bucket()` (Postgres path); simpler bucketing or a documented
   limitation on the SQLite path.

2. (Later, as §11 progresses) any aggregate endpoints Grafana panels rely on, added one at a
   time as the panel absorbs each dashboard.

Each addition follows repo rules: parameterized SQL, `anyhow::Result`, `tracing`, tests on
values, `just ci` green.

## 10. "Add server" flow (v1)

The panel does **not** SSH from the browser in v1. Instead:

1. Operator clicks "Add server".
2. API fetches the org master's register token (`GET /api/register-token`) + endpoint.
3. UI renders the ready-to-paste install command:

   ```
   curl -sfL .../install-pulse.sh | \
     PULSE_MASTER=https://MASTER:50051 PULSE_TOKEN=<register-token> GITHUB_TOKEN=... sudo sh -
   ```

4. Operator runs it on the target server; the agent auto-registers and appears in the fleet.

**Phase 2 (out of scope now):** API executing `ssh`/`scp` to provision remotely, mirroring
`pulse-cli`'s `installer.rs`. Flagged as security-sensitive.

## 11. Grafana sunset roadmap

The intent is to **remove Grafana entirely** once the panel covers its visualization:

1. **v1** — control plane + basic live charts (poll `/metrics/latest`).
2. **v2** — time-range charts via the range endpoint (§9.1); begins Grafana core parity.
3. **v3** — port remaining `grafana/dashboard*.json` panels (per-core, GPU, containers, TCP);
   add any missing aggregate endpoints (§9.2).
4. **Decommission** — remove the `grafana` service from `docker-compose.yml`, the `grafana/`
   provisioning, and `docs/grafana.md`; update README. Only after real-parity.

Until step 4 both coexist, reading the same TimescaleDB data (Grafana directly, panel via API).

## 12. Repository layout

The Cargo workspace at the repo **root stays untouched** (it is its own build system). All
TypeScript lives under a single top-level **`cloud/`** directory, which is **one Bun
workspace** — kept isolated so the whole thing is one clean `git subtree split --prefix=cloud`
away from becoming its own repo when a trigger appears (§ below).

Convention: **Rust in `crates/`** (root), **all TypeScript in `cloud/`** split into runnable
`apps/*` and shared `packages/*` — every cross-cutting concern (db, contracts, auth) is its
own package so nothing is duplicated across apps.

```
/  (repo root = Cargo workspace, unchanged)
├── Cargo.toml, crates/, migrations/, docker/ ...   # all Rust stays in crates/
└── cloud/                      # 1 Bun workspace · 1 extractable subtree
    ├── package.json            # { "workspaces": ["apps/*", "packages/*"] }
    ├── biome.json              # shared lint/format config for the whole TS side
    ├── apps/
    │   ├── server/             # Elysia + Bun — the service API
    │   │   ├── package.json
    │   │   └── src/
    │   │       ├── index.ts    # Elysia app: swagger, cors, rate-limit, security headers
    │   │       ├── master.ts   # per-org pulse-master REST client
    │   │       ├── routes/     # v1: servers, metrics, alerts, webhooks, auth mount
    │   │       └── webhooks/   # subscription matching + signed dispatch + delivery log
    │   └── web/                # SvelteKit + Bun — the web UI (consumes apps/server)
    │       ├── package.json
    │       ├── tailwind.config.ts
    │       └── src/
    │           ├── lib/
    │           │   ├── api/        # typed client re-exported from packages/contracts
    │           │   ├── components/
    │           │   │   ├── ui/     # Bits UI + Tailwind primitives (Button, Card, Dialog…)
    │           │   │   └── <feature>/  # fleet, agent, alerts, webhooks
    │           │   ├── hooks/      # global hooks — useAgents, usePolling, useAuth, useToast
    │           │   └── stores/     # shared reactive state (one source of truth)
    │           └── routes/         # login, fleet, agent detail, add-server, alerts, webhooks
    └── packages/
        ├── db/                 # ALL database code: Drizzle schema + migrations + client
        │                       #   (better-auth tables, orgs, webhooks, audit). One source of truth.
        ├── auth/               # better-auth config: server instance + client (org/admin/apiKey)
        └── contracts/          # shared types + typed SDK derived from Elysia/OpenAPI
                                #   (consumed by both apps/server and apps/web)
```

- **Root stays Rust.** `cloud/` is out of the Cargo workspace and Rust CI; it has its own
  Bun-based lint/test/build CI job, gated by a `cloud/**` path filter.
- **Bun workspaces (not Turborepo yet).** Native workspace linking is enough at this size.
  Turbo/Nx is deferred until the TS side grows further **and** CI task-caching pays off — and
  even then it would be scoped inside `cloud/`, never at root.
- **`packages/db`** owns all schema/migrations/client, so `server` (and any future worker,
  seed or script) share one source of truth for the database.
- **`packages/auth`** centralizes the better-auth config (server + client) so web and server
  never duplicate it.
- **`packages/contracts`** is the single source of truth for request/response types and the
  typed API client, so `server` and `web` can never drift.
- `node_modules`, build output and `.env` files added to `.gitignore`.

### When to split into a separate repo

Extraction is cheap because everything is under `cloud/`. Natural triggers:

- **Divergent license** — the repo is MIT today; if the SaaS goes closed-source, split.
- **Separate team / access control / deploy cadence** for the cloud product.
- **Open-sourcing the agent/master** while keeping the cloud private.

Until one of those holds, the isolated monorepo is the more productive setup.

## 13. Configuration (env)

```
# cloud/apps/server
DATABASE_URL=postgres://pulse:pulse@localhost:5432/pulse  # Postgres for better-auth + orgs + webhooks + audit — packages/db
BETTER_AUTH_SECRET=...                # better-auth signing secret
BETTER_AUTH_URL=http://localhost:3000 # API origin
CORS_ORIGINS=http://localhost:5173    # allow-list for the UI
ALERT_INGEST_SECRET=...               # shared secret for POST /v1/internal/alerts
# Per-org master URL + admin token live in the DB (organizations table), not in env.
# A single bootstrap master (URL + token) may be set here only for local dev.

# cloud/apps/web
PUBLIC_API_URL=http://localhost:3000  # base URL of the API
```

## 14. Security considerations

- Admin/master tokens confined to the API server; never serialized to client payloads.
- httpOnly + Secure + SameSite cookies and password hashing handled by better-auth.
- All input validated by TypeBox schemas; typed error envelope leaks no internals.
- Rate limiting per IP and per API key; CORS allow-list; security headers globally.
- Webhook payloads HMAC-signed per subscription; ingest endpoint requires `ALERT_INGEST_SECRET`.
- `exec` is high-risk (arbitrary commands) — gate behind role, and audit-log every call
  (who, org, agent, command, timestamp).
- Tenant isolation (Model A): every query scoped by `org_id`; master resolved per org.
- Never log passwords, session ids, tokens, API keys, or command output containing secrets.

## 15. Open questions / to confirm

- **Org → master provisioning (SaaS):** when a customer signs up, how does their org get a
  master? Options: (a) manual/assisted per org for v1; (b) auto-provision a master container
  per org; (c) pooled masters. Start with (a); design (b) next.
- Deployment shape: ship API + panel as containers in `docker-compose.yml`, or dev-only first?
- Downsampling contract for the range endpoint: fixed steps vs. server-chosen bucket.
- Billing/metering provider (Stripe?) and what we meter (agents, retention, seats).

## 16. Proposed milestones

- **M0 — done.** Scaffolded the `cloud/` Bun workspace: `apps/server` (Elysia + Swagger at
  `/docs` + CORS + security headers + `/health` + `/v1`), `apps/web` (SvelteKit + Tailwind 4
  + hooks/stores + typed client), and `packages/{db,auth,contracts}`. better-auth wired
  (organization + admin plugins), per-org master client, 20 tests, Biome + TS strict green,
  isolated `cloud/**` CI job.
- **M1** — Auth + tenancy: better-auth (email/password, org plugin), bootstrap owner,
  login/logout, session + org middleware, API keys.
- **M2** — Servers API + UI: fleet overview, agent detail (live charts), rename/remove/exec,
  fully validated + documented in Swagger.
- **M3** — Alerts + **webhooks**: subscriptions (org/server/user), signed dispatch, delivery
  log, test-send.
- **M4** — Add-server flow (command + register token).
- **M5** — Range endpoint in Rust (§9.1) + time-range charts (Grafana parity begins).
- **M6+** — Grafana sunset per §11; billing/metering hooks if SaaS.
```
