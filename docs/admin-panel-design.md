# Admin Panel — Design Document

> Status: **Proposal / awaiting approval.** No application code has been scaffolded yet.
> This document captures the agreed architecture so implementation can start from a shared plan.

## 1. Motivation

Vixpi Pulse today has three faces: the `pulse` TUI (operators), pre-built Grafana
dashboards (visualization), and the `pulse-master` REST API (the machine interface).
There is **no multi-user web control plane**.

The goal is a web **admin panel** that:

- Authenticates real users (login, registration, sessions) — the master only knows a
  single shared admin token today.
- Manages the fleet from a browser: list servers, view metrics, rename/remove agents,
  run remote commands, view/acknowledge alerts.
- Onboards new servers by generating the install command + register token.
- **Eventually replaces Grafana** — see §8. Grafana stays for now; the panel grows until
  it covers the same visualization needs, then Grafana is dropped.

### Explicit non-goals for v1

- Full RBAC / teams / orgs. Start with a single role (admin). Design leaves room to grow.
- SSH-based remote provisioning from the browser (phase 2 — see §7).
- Re-implementing every Grafana dashboard on day one (that is the §8 roadmap, not v1).

## 2. Core decision: keep user management OUT of Rust

`pulse-master` is a lean telemetry engine (5.8 MB, zero external deps, zero `unsafe`).
Adding user accounts, sessions, password hashing and RBAC to it would betray that design.

Instead, the panel is a **separate TypeScript application** that acts as a
**Backend-for-Frontend (BFF)**:

```
                cookie session                  Bearer PULSE_ADMIN_TOKEN
  Browser ───────────────────────▶ SvelteKit ──────────────────────────▶ pulse-master
  (user login)                     (server, Bun)      (server-to-server)   REST :8080
                                        │
                                        └── bun:sqlite  (users, sessions)
```

Consequences:

- The `PULSE_ADMIN_TOKEN` **never reaches the browser.** It lives only in the SvelteKit
  server process, which calls the master machine-to-machine. Browser clients hold only an
  httpOnly session cookie tied to a panel user.
- The user layer lives in TypeScript, where it is cheap and disposable — exactly where we
  want something that may be extracted or replaced later.
- **The Rust master is unchanged** except for a few small, surgical read endpoints (§6).
  No user/auth logic ever enters Rust.

### Why not Clerk / an external auth SaaS

Vixpi Pulse's whole premise is "runs inside your server, zero external dependencies."
Depending on an auth SaaS to monitor self-hosted infrastructure is contradictory and adds
an outbound dependency + failure mode. We roll a minimal self-hosted auth instead
(§4). This is a deliberate rejection of Clerk/Auth0/etc.

## 3. Tech stack

| Concern      | Choice                          | Rationale                                             |
|--------------|---------------------------------|-------------------------------------------------------|
| Runtime      | **Bun**                         | Fast, native `bun:sqlite`, single toolchain           |
| Framework    | **SvelteKit** (adapter-node/bun)| Full-stack — server routes make the BFF trivial       |
| Lint/format  | **BiomeJS**                     | Single fast tool, no ESLint+Prettier sprawl           |
| Auth         | Self-hosted sessions            | httpOnly cookie + argon2 password hash                 |
| Panel DB     | **bun:sqlite** (+ Drizzle)      | Users/sessions only; tiny, embedded, no server        |
| Charts       | **uPlot**                       | ~40 KB, built for time series; matches the lean ethos |

Dependency discipline mirrors the Rust side: add a dependency only with justification.
No component library bloat; hand-rolled minimal UI to start.

## 4. Authentication design (v1)

- **Registration:** first-run bootstrap creates the initial admin. Further registration is
  invite/admin-gated (open self-registration is off by default for an infra tool).
- **Passwords:** hashed with argon2id. Never logged, never returned.
- **Sessions:** opaque random session id stored in `bun:sqlite`, delivered as an httpOnly,
  SameSite=Lax, Secure cookie. Server validates on every request in `hooks.server.ts`.
- **Authorization to the master:** the SvelteKit server attaches `PULSE_ADMIN_TOKEN`
  (from its own env) to outbound master calls. Panel users never see or hold it.
- **CSRF:** SvelteKit's built-in origin check for form actions; state-changing API routes
  require same-origin + valid session.

Schema (panel DB, indicative):

```
users(id, email, password_hash, role, created_at)
sessions(id, user_id, expires_at, created_at)
```

## 5. What the panel calls (existing master API)

The master already exposes everything the control plane needs to **read and operate**:

- Read: `/api/overview`, `/api/agents`, `/api/agents/:id/metrics[/latest]`, `/disks`,
  `/network`, `/containers`, `/processes`, `/gpu`, `/sessions`, `/connections`, `/api/alerts`.
- Manage: `DELETE /api/agents/:id`, `PUT /api/agents/:id/name`, `POST /api/agents/:id/exec`,
  `GET /api/register-token`, updates rollout endpoints.

The SvelteKit server wraps these behind its own `/api/*` (or load functions), adding the
session check and injecting the admin token.

## 6. Required Rust additions (minimal, surgical)

Approved: small read-only additions to the master API. No user logic in Rust.

1. **Time-range metrics endpoint** — the current API only returns the last ~20 readings.
   Grafana-style charts and Grafana replacement (§8) need windowed, downsampled series:

   ```
   GET /api/agents/:id/metrics?from=<ts>&to=<ts>&step=<seconds>&fields=cpu,mem,...
   ```

   Backed by TimescaleDB `time_bucket()` for aggregation/downsampling (Postgres path).
   For the SQLite path, a simpler bucketed query or a documented limitation.

2. (Later, as §8 progresses) any aggregate endpoints Grafana panels rely on that aren't yet
   expressible, added one at a time as the panel absorbs each dashboard.

Each addition follows repo rules: parameterized SQL, `anyhow::Result`, `tracing`, tests on
values, `just ci` green.

## 7. "Add server" flow (v1)

The panel does **not** SSH from the browser in v1. Instead:

1. Operator clicks "Add server".
2. Panel fetches the register token (`GET /api/register-token`) and the master endpoint.
3. Panel renders the ready-to-paste install command, e.g.:

   ```
   curl -sfL .../install-pulse.sh | \
     PULSE_MASTER=https://MASTER:50051 PULSE_TOKEN=<register-token> GITHUB_TOKEN=... sudo sh -
   ```

4. Operator runs it on the target server; the agent auto-registers and appears in the fleet.

**Phase 2 (out of scope now):** SvelteKit server executing `ssh`/`scp` to provision
remotely, mirroring `pulse-cli`'s `installer.rs`. Flagged as security-sensitive.

## 8. Grafana sunset roadmap

The panel is the control plane first, but the intent is to **remove Grafana entirely** once
the panel covers its visualization. Sequencing:

1. **v1** — control plane + basic live charts (poll `/metrics/latest`, small windows).
2. **v2** — time-range charts via the new range endpoint (§6.1): CPU/mem/disk/net over
   selectable windows with downsampling. This is the point where the panel starts matching
   Grafana's core dashboards.
3. **v3** — port the remaining `grafana/dashboard*.json` panels (per-core, GPU, containers,
   TCP) into the panel. Add whatever aggregate endpoints are still missing (§6.2).
4. **Decommission** — remove the `grafana` service from `docker-compose.yml`, the
   `grafana/` provisioning, and `docs/grafana.md`; update README. Only after the panel is at
   parity for the dashboards in active use.

Until step 4, Grafana and the panel coexist and read the same TimescaleDB-backed data
(Grafana directly, panel via the master API).

## 9. Repository layout

The panel lives in the repo **temporarily**, isolated so it can be extracted later:

```
apps/panel/            # SvelteKit + Bun app (self-contained)
  package.json
  biome.json
  drizzle/             # panel DB schema + migrations (users/sessions)
  src/
    hooks.server.ts    # session validation, admin-token injection
    lib/server/        # master API client, auth, db
    routes/            # login, fleet, agent detail, add-server, alerts
```

- Kept **out of the Cargo workspace** and Rust CI. Its own lint/test job.
- `node_modules` and the panel's local SQLite file added to `.gitignore`.
- Self-contained so a future `git subtree split` / move to another repo is clean.

## 10. Configuration (panel env)

```
PULSE_MASTER_API=http://master:8080     # base URL of pulse-master REST API
PULSE_ADMIN_TOKEN=...                    # server-side only; injected into master calls
PANEL_DB_PATH=./panel.sqlite             # bun:sqlite file for users/sessions
PANEL_SESSION_SECRET=...                 # cookie signing / session secret
PANEL_ALLOW_OPEN_REGISTRATION=false      # default: admin-gated
```

## 11. Security considerations

- Admin token confined to the SvelteKit server; never serialized to client payloads.
- httpOnly + Secure + SameSite cookies; argon2id password hashing.
- Same-origin enforcement on state-changing routes; SvelteKit CSRF for form actions.
- `exec` is a high-risk surface (arbitrary commands) — gate behind admin role and audit-log
  every invocation (who, agent, command, timestamp) in the panel DB.
- Never log passwords, session ids, the admin token, or command output containing secrets.

## 12. Open questions / to confirm before v2+

- Multi-user beyond a single admin: do we need viewer vs admin roles in v1, or is one role
  enough until the panel is in real use?
- Deployment shape: ship the panel as a container alongside `master` in
  `docker-compose.yml`, or keep it dev-only until it stabilizes?
- Downsampling contract for the range endpoint: fixed steps vs. server-chosen bucket based
  on `to-from`.

## 13. Proposed milestones

- **M0** — Scaffold `apps/panel` (SvelteKit + Bun + Biome), health check, master API client,
  isolated CI job. (Awaiting go-ahead.)
- **M1** — Auth: bootstrap admin, login/logout, session middleware, protected routes.
- **M2** — Fleet overview + agent detail (live charts) + rename/remove/exec + alerts view.
- **M3** — Add-server flow (command + register token).
- **M4** — Range endpoint in Rust (§6.1) + time-range charts (Grafana-parity begins).
- **M5+** — Grafana sunset per §8.
```
