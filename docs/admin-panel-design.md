# Admin Panel & Service API — Design Document

> Status: **Proposal / awaiting approval.** No application code has been scaffolded yet.
> This document captures the agreed architecture so implementation can start from a shared plan.

## 1. Motivation

Vixpi Pulse today has three faces: the `pulse` TUI (operators), pre-built Grafana
dashboards (visualization), and the `pulse-master` REST API (the machine interface).
There is **no multi-user web control plane**, and no productized, documented service API.

Two goals now drive this work:

1. A web **admin panel** — multi-user control plane to manage servers, view metrics, onboard
   agents, run commands, and view/configure alerts from a browser.
2. A sellable **service** — the intent is to offer Pulse to other people. That means a
   documented public **API** (Swagger), real authentication, input validation, security
   middleware, per-customer webhooks, and (potentially) multi-tenancy. See §4.

### Explicit non-goals for v1

- Billing/metering integration (Stripe etc.) — designed around, not built yet.
- SSH-based remote provisioning from the browser (phase 2 — see §10).
- Re-implementing every Grafana dashboard on day one (that is the §11 roadmap, not v1).

## 2. Core decision: keep user/tenant logic OUT of Rust

`pulse-master` is a lean telemetry engine (5.8 MB, zero external deps, zero `unsafe`).
Adding accounts, sessions, RBAC, tenancy and webhooks to it would betray that design.

Instead, all of that lives in a TypeScript layer split into two apps:

- **`apps/api` — Elysia (Bun):** the product API and gateway. Owns auth, input validation,
  Swagger docs, security middleware, webhook subscriptions + dispatch, and the only client
  that talks to `pulse-master` (holding its admin token). This is the sellable, documented
  surface.
- **`apps/panel` — SvelteKit (Bun):** the web UI. Consumes `apps/api`; holds no secrets.

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
          pulse-master REST :8080          bun:sqlite + Drizzle
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
| Runtime       | **Bun**                         | Fast, native `bun:sqlite`, single toolchain for both apps  |
| API framework | **Elysia**                      | Bun-native; end-to-end typed; TypeBox validation + Swagger built into the route definitions |
| Validation    | **TypeBox** (Elysia `t`)        | One schema drives runtime validation, TS types, and OpenAPI |
| API docs      | **@elysiajs/swagger**           | OpenAPI/Swagger generated from the route schemas — always in sync |
| Web UI        | **SvelteKit** (adapter-node/bun)| Client for the API; fast, minimal                          |
| Lint/format   | **BiomeJS**                     | Single fast tool, no ESLint+Prettier sprawl                |
| Auth          | **better-auth**                 | Self-hosted; email/password, sessions, plus `organization`, `admin`, `apiKey` plugins for tenancy/RBAC/programmatic access |
| App DB        | **bun:sqlite** (+ Drizzle)      | Backs better-auth + orgs/webhooks/audit; embedded, no server |
| Charts        | **uPlot**                       | ~40 KB, built for time series; matches the lean ethos      |

Dependency discipline mirrors the Rust side: add a dependency only with justification.

## 4. Multi-tenancy & productization (needs a decision)

Selling Pulse forces one architectural fork. Two viable models:

**Model A — Hosted multi-tenant SaaS.** We run the API; customers sign up, each in their own
**organization**. An org's servers/metrics/webhooks are isolated from every other org. Since
`pulse-master` is single-tenant, the clean isolation is **one master per organization**: the
`organizations` table stores that org's master URL + admin token, and the API routes each
request to the caller's org master. Hard data isolation, master stays simple.

**Model B — Self-hosted licensed product.** Each customer runs the whole stack (master + API
+ panel) themselves; single tenant per install. Selling = licensing/packaging. No tenant
scoping needed in the schema.

**Recommendation (hedges both):** build on better-auth's **`organization` plugin from day
one**, where every user belongs to an org. Self-hosted deployments simply have exactly one
org; SaaS has many. Master access is resolved **per org** (URL + token on the org record).
This costs little now and avoids a painful retrofit if we go SaaS later.

> This is the one decision that changes the data model materially — confirmed choice needed
> before M1.

## 5. Authentication & authorization

Auth is handled by **better-auth**, mounted inside the Elysia API (`/api/auth/*`) against the
app DB via its Drizzle adapter. The SvelteKit UI uses the better-auth client against that
same origin.

- **Providers:** email & password to start. Social/SSO later via better-auth providers if
  customers ask.
- **Registration:** open sign-up **disabled by default** for self-hosted; for SaaS, sign-up
  creates a new org (org owner). Gated via better-auth config / the `admin` plugin.
- **Passwords:** hashed by better-auth (scrypt default). Never logged or returned.
- **Sessions:** DB-backed, delivered as httpOnly, SameSite=Lax, Secure cookies; validated on
  every request via better-auth (`getSession`) populating request context / `event.locals`.
- **Roles / RBAC:** better-auth `admin` + `organization` plugins provide roles (owner/admin/
  viewer) and per-org membership without a schema rewrite.
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

## 7. Web UI (SvelteKit)

Thin client over the API: login, fleet overview, agent detail (live + range charts),
rename/remove/exec, alerts, webhook management, add-server. Holds no secrets — all data and
mutations go through the API with the session cookie.

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

Both apps live in the repo **temporarily**, isolated so they can be extracted later:

```
apps/
  api/                 # Elysia + Bun — the service API
    package.json
    biome.json
    drizzle/           # app DB schema + migrations (better-auth, orgs, webhooks, audit)
    src/
      index.ts         # Elysia app: swagger, cors, rate-limit, security headers
      auth.ts          # better-auth instance (Drizzle adapter, org/admin/apiKey plugins)
      db.ts            # bun:sqlite + Drizzle
      master.ts        # per-org pulse-master REST client
      routes/          # v1: servers, metrics, alerts, webhooks, auth mount
      webhooks/        # subscription matching + signed dispatch + delivery log
  panel/               # SvelteKit + Bun — the web UI (consumes apps/api)
    package.json
    biome.json
    src/routes/        # login, fleet, agent detail, add-server, alerts, webhooks
```

- Kept **out of the Cargo workspace** and Rust CI. Their own lint/test job.
- `node_modules` and local SQLite files added to `.gitignore`.
- Self-contained so a future `git subtree split` / move to another repo is clean.

## 13. Configuration (env)

```
# apps/api
PULSE_ADMIN_TOKEN=...                 # fallback/self-hosted single-org master token (server-only)
PULSE_MASTER_API=http://master:8080   # fallback/self-hosted single-org master URL
APP_DB_PATH=./pulse-panel.sqlite      # bun:sqlite (better-auth + orgs + webhooks + audit)
BETTER_AUTH_SECRET=...                # better-auth signing secret
BETTER_AUTH_URL=http://localhost:3000 # API origin
CORS_ORIGINS=http://localhost:5173    # allow-list for the UI
ALERT_INGEST_SECRET=...               # shared secret for POST /v1/internal/alerts
# (SaaS/multi-tenant: master URL + token live per-org in the DB, not here)

# apps/panel
PUBLIC_API_URL=http://localhost:3000  # base URL of apps/api
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

- **Selling model (blocking, §4):** Model A (hosted multi-tenant SaaS) or Model B (self-hosted
  licensed)? Recommendation: build org-scoped from day one to serve both.
- Roles needed at launch: owner/admin/viewer, or just admin until real use?
- Deployment shape: ship API + panel as containers in `docker-compose.yml`, or dev-only first?
- Downsampling contract for the range endpoint: fixed steps vs. server-chosen bucket.

## 16. Proposed milestones

- **M0** — Scaffold `apps/api` (Elysia + Bun + Biome + Swagger skeleton) and `apps/panel`
  (SvelteKit), health checks, per-org master client, isolated CI job. (Awaiting go-ahead.)
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
