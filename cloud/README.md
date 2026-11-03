# Vixpi Pulse — Cloud (SaaS control plane)

The TypeScript side of Vixpi Pulse: a hosted, multi-tenant web control plane + service API on
top of the Rust `pulse-master` telemetry engine. One Bun workspace, extractable as a subtree.

> Design: [`../docs/admin-panel-design.md`](../docs/admin-panel-design.md) ·
> Contributor rules: [`AGENTS.md`](./AGENTS.md)

## Layout

| Path                  | What                                                      |
| --------------------- | -------------------------------------------------------- |
| `apps/server`         | Elysia API — auth, validation, Swagger, webhooks, gateway |
| `apps/web`            | SvelteKit UI (Bits UI + Tailwind) — thin client          |
| `packages/db`         | Drizzle schema + migrations + client (`bun:sqlite`)      |
| `packages/auth`       | better-auth config (server + client)                     |
| `packages/contracts`  | shared types + typed API client                          |

## Develop

```bash
bun install

bun run --filter '@vixpi/server' dev   # API on :3000  (Swagger at /docs)
bun run --filter '@vixpi/web' dev       # UI on :5173

bun run check                            # lint + typecheck + tests (all workspaces)
```

Requires [Bun](https://bun.sh) >= 1.3. The Rust master lives at the repo root — see the top-level README.

## CI

The workflow lives at [`ci/cloud.yml`](./ci/cloud.yml). Install it by copying to
`.github/workflows/cloud.yml` (kept out of `.github/` here because the automation token lacks
the `workflow` scope to push it). It runs `lint → typecheck → test`, gated on `cloud/**`.
