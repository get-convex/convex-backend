# `convex-orchestrator`: a self-hosted BigBrain replacement

> **Status:** experimental in this fork. The orchestrator implements the same
> HTTP surface that the hosted Convex "BigBrain" provisioning service exposes,
> so the existing Convex CLI, dashboard, and `crates/big_brain_client` work
> unmodified against it.

## Three-service architecture

Self-hosted Convex with the orchestrator runs three services:

1. **`convex-orchestrator`** (this package, Rust binary from
   `crates/orchestrator`). The "platform" service: members, teams, projects,
   deployments, deploy keys. Stores its state in PostgreSQL. Listens on `:8050`
   by default.
2. **`dashboard-orchestrator`** (Next.js app in
   `npm-packages/dashboard-orchestrator`). The platform UI: login, list teams,
   list projects, list deployments, provision new deployments. Listens on
   `:6792` (`next dev`) or `:6793` (`next start`). Talks to (1) over HTTP.
3. **`convex-local-backend`** instances, one per logical deployment, each with
   its own SQLite/Postgres database and admin key. Per-deployment dashboard UI
   (data, functions, logs, etc.) is the existing **`dashboard-self-hosted`**
   package — the orchestrator dashboard embeds it via iframe and passes the
   admin key with the same `postMessage` protocol the dashboard already supports
   for embedded scenarios.

## What it is

The Convex Cloud product has two services:

- The **backend** — `convex-local-backend`, the runtime that serves queries,
  mutations, actions, and stores your data. Open-source, included in this
  repository.
- The **orchestrator** ("BigBrain") — provisions deployments, manages teams,
  projects, members, deploy keys, billing, and audit logs. Closed-source in
  Convex Cloud.

Self-hosted deployments traditionally run only the backend. That gives you a
single deployment per host, no team/project model, and no `npx convex dev`-style
provisioning workflow.

`convex-orchestrator` is a self-hosted, open-source replacement for BigBrain.
With it, the same dashboard and CLI you'd use against Convex Cloud point at your
own infrastructure and behave the same way.

## What's included vs stubbed

**Included (works end-to-end):**

- Members, teams, projects, deployments.
- Personal access tokens, team access tokens, deploy keys (prod / dev / preview
  / project-level).
- The full deployment-internal credential exchange used by the CLI and the Rust
  `big_brain_client` (`/api/deployment/authorize_*`,
  `/api/deployment/provision_and_authorize`,
  `/api/deployment/team_and_project*`, etc.).
- The dashboard private API (`/api/dashboard/...`).
- The public Convex Management API (`/v1/...`).
- Audit log.
- Default project-level environment variables.
- Custom domain registration (records only — certificate provisioning is left to
  your reverse proxy).

**Stubbed** (returns sensible empty / defaults so the dashboard renders without
errors, but the cloud-only feature is not implemented):

- Orb subscriptions, plans, invoices, spending limits, payment methods.
- Discord linking.
- Vercel marketplace.
- WorkOS / SSO / AuthKit.
- Cloud backups (self-hosted users use the existing `npx convex export` /
  `npx convex import` flow on the backend instead).
- Periodic backup scheduling.
- Databricks-backed usage analytics.
- OAuth-app registration on the platform itself.

See `docs/superpowers/specs/2026-05-02-convex-orchestrator-design.md` for the
complete design and reasoning.

## Running the orchestrator

The orchestrator stores its state in PostgreSQL. We recommend
[PlanetScale Postgres](https://planetscale.com/postgres) (the same product
Convex Cloud uses for BigBrain) but any Postgres 14+ instance works.

```sh
# Build
cargo build --release -p orchestrator

# Run, pointing at your Postgres
export CONVEX_ORCHESTRATOR_DATABASE_URL="postgres://user:pass@host:5432/orchestrator?sslmode=require"
./target/release/convex-orchestrator \
  --bootstrap-token "$(openssl rand -hex 32)"
```

The orchestrator runs its schema migrations automatically on startup. For
PlanetScale (and any provider that requires TLS), include `sslmode=require` in
the connection URL — the orchestrator reads it and wraps the connection with
rustls.

The orchestrator listens on `0.0.0.0:8050` (BigBrain's historical port). On
first start with `--bootstrap-token` set, it creates an owner member
`admin@self-hosted.local`, a default team `self-hosted`, and registers the
bootstrap token as a Personal Access Token for that member.

You can then point the CLI at the orchestrator:

```sh
export CONVEX_PROVISION_HOST="http://localhost:8050"
export CONVEX_OVERRIDE_ACCESS_TOKEN="<your bootstrap token>"
npx convex --help          # CLI now talks to your orchestrator
```

And the dashboard (`npm-packages/dashboard/`):

```sh
NEXT_PUBLIC_BIG_BRAIN_URL=http://localhost:8050 npm run dev
```

## Provisioner modes

`--provisioner external` (default): the operator pre-starts backend instances
out-of-band (e.g. via `docker compose`) and registers them via the
orchestrator's `POST /api/dashboard/deployments/register` endpoint. The
orchestrator only stores metadata and admin-key hashes.

`--provisioner process`: the orchestrator allocates a port range, spawns a
`convex-local-backend` per logical deployment, mints its admin key, and
supervises the process. v1 of this fork ships only the credential-minting half
of process-mode; actually launching `convex-local-backend` from the orchestrator
is not yet automated and you must start it yourself with the printed CLI args.

## Storage

The orchestrator stores its own state (members, teams, projects, deployments,
access tokens, audit log) in PostgreSQL — same as Convex Cloud. The
`CONVEX_ORCHESTRATOR_DATABASE_URL` env var (or `--database-url`) is required.

Per-deployment backend data — the actual user data your apps store — lives in
each `convex-local-backend` instance's own SQLite or Postgres database, which
the orchestrator manages independently of its own DB.

## OpenAPI

The orchestrator's wire format mirrors the dashboard's existing TypeScript
clients. Run

```sh
./target/release/convex-orchestrator --print-openapi > openapi.json
```

to dump a spec compatible with the dashboard's
`dashboard-management-openapi.json` (subset).

## Where it lives in this repo

- `crates/orchestrator/` — Rust binary + library (binary name
  `convex-orchestrator`)
- `crates/orchestrator_api_types/` — request / response DTOs
- `npm-packages/dashboard-orchestrator/` — Next.js platform UI
- `docs/superpowers/specs/2026-05-02-convex-orchestrator-design.md` — design
- `docs/superpowers/specs/2026-05-02-convex-orchestrator-plan.md` —
  implementation plan

## Running the dashboard

```sh
cd npm-packages/dashboard-orchestrator
NEXT_PUBLIC_CONVEX_ORCHESTRATOR_URL=http://localhost:8050 \
NEXT_PUBLIC_SELF_HOSTED_DASHBOARD_URL=http://localhost:6791 \
  npm run dev
```

Open `http://localhost:6792`. Sign in using the `--bootstrap-token` value you
passed to the orchestrator on first start; you'll land on the team list, then
pick or create a project, then pick or provision a deployment. The deployment
view embeds `dashboard-self-hosted` (running on `:6791`) for the per-deployment
data/functions/logs/etc. UI.
