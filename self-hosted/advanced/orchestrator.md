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

## Deploying with Docker Compose

The orchestrator, Postgres, the platform UI, and the per-deployment dashboard
are wired up in
[`docker-compose.orchestrator.yml`](https://github.com/defy-works/convex-backend/tree/release/self-hosted/docker/docker-compose.orchestrator.yml).
Every value is configurable via the environment. With no `.env` it boots against
`http://localhost`; for anything beyond local dev, point browsers at your domain
and replace the three secrets.

Download the compose file onto your server:

```sh
curl -O https://raw.githubusercontent.com/defy-works/convex-backend/release/self-hosted/docker/docker-compose.orchestrator.yml
curl -O https://raw.githubusercontent.com/defy-works/convex-backend/release/self-hosted/docker/init-better-auth.sql
```

Set up routing to forward requests from your domain to the four host ports
exposed by the stack:

- `https://convex.my-domain.com` forwards to `http://localhost:6793` — the
  platform UI (sign in, teams, projects, deployments).
- `https://api.convex.my-domain.com` forwards to `http://localhost:8050` — the
  orchestrator API.
- `https://embed.convex.my-domain.com` forwards to `http://localhost:6791` — the
  per-deployment dashboard, embedded as iframes.
- `https://*.convex.my-domain.com` forwards to `http://localhost:9000` — the
  router that fronts each spawned `convex-backend`. Each deployment is reached
  at `<deployment-id>.convex.my-domain.com` and
  `<deployment-id>-site.convex.my-domain.com`, so a wildcard DNS + TLS record is
  required.

In a `.env` file beside the compose file:

```sh
# Secrets — generate with `openssl rand -hex 32`. The defaults are
# placeholders for local dev only.
BOOTSTRAP_TOKEN='<random-hex>'
SERVICE_KEY='<random-hex>'
BETTER_AUTH_SECRET='<random-hex>'

# Public URLs the browser uses. The dashboard server reads these at
# request time and injects them into the page via _document.tsx, so the
# pre-built image runs unchanged on any host.
PUBLIC_ORIGIN='https://convex.my-domain.com'
PUBLIC_DASHBOARD_URL='https://convex.my-domain.com'
PUBLIC_ORCHESTRATOR_URL='https://api.convex.my-domain.com'

# Spawned-deployment subdomains. Points at the wildcard record above.
ROUTER_HOST='convex.my-domain.com'
ROUTER_PUBLIC_PORT='443'

# First admin email + signup policy. `allowlist` = only ADMIN_EMAILS can
# sign up; `open` = anyone; `closed` = no signup (invite-only).
ADMIN_EMAILS='you@my-domain.com'
REGISTRATION_MODE='allowlist'
```

Start the stack:

```sh
docker compose -f docker-compose.orchestrator.yml up -d
```

Sign in at `https://convex.my-domain.com` using `BOOTSTRAP_TOKEN` (the
orchestrator registers it as a personal access token for the first admin on
first start).

### Environment reference

| Var                                                                                                                  | Default                                                                            | Purpose                                                                                                                               |
| -------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `BOOTSTRAP_TOKEN`                                                                                                    | `orchestrator-bootstrap-token-change-me`                                           | Initial admin PAT. Change before exposing the stack.                                                                                  |
| `SERVICE_KEY`                                                                                                        | `orchestrator-service-key-change-me`                                               | Shared secret between orchestrator and dashboard for `/api/internal/*`.                                                               |
| `BETTER_AUTH_SECRET`                                                                                                 | `better-auth-secret-change-me`                                                     | Session signing key for the dashboard auth.                                                                                           |
| `PUBLIC_ORIGIN`                                                                                                      | `http://localhost`                                                                 | Stamped into URLs the orchestrator returns to clients.                                                                                |
| `PUBLIC_DASHBOARD_URL`                                                                                               | `http://localhost:6793`                                                            | Browser URL of the platform UI; also `BETTER_AUTH_URL` and the inner dashboard's `TRUSTED_PARENT_ORIGINS`.                            |
| `PUBLIC_ORCHESTRATOR_URL`                                                                                            | `http://localhost:8050`                                                            | Browser-facing orchestrator URL. Read by the dashboard server at request time and injected into the page; no image rebuild required.  |
| `ROUTER_HOST`                                                                                                        | `localhost`                                                                        | Suffix host for spawned-deployment subdomains.                                                                                        |
| `ROUTER_PUBLIC_PORT`                                                                                                 | `${ROUTER_PORT}`                                                                   | Public port for the router (set to `443` behind TLS).                                                                                 |
| `ORCHESTRATOR_PORT` / `ROUTER_PORT` / `DASHBOARD_ORCHESTRATOR_PORT` / `DASHBOARD_SELF_HOSTED_PORT` / `POSTGRES_PORT` | `8050` / `9000` / `6793` / `6791` / `5433`                                         | Host port bindings.                                                                                                                   |
| `POSTGRES_USER` / `POSTGRES_PASSWORD` / `POSTGRES_DB`                                                                | `orchestrator`                                                                     | Bundled Postgres credentials.                                                                                                         |
| `CONVEX_ORCHESTRATOR_DATABASE_URL` / `BETTER_AUTH_DATABASE_URL`                                                      | derived from the bundled Postgres                                                  | Override to point at an external Postgres.                                                                                            |
| `PROVISIONER`                                                                                                        | `docker`                                                                           | `docker` shells out to the host docker socket; `external` is a no-op (register backends manually); `process` is for embedded testing. |
| `BACKEND_IMAGE`                                                                                                      | `ghcr.io/defy-works/convex-backend:latest`                                         | Image the provisioner runs for each new deployment.                                                                                   |
| `BACKEND_CONTAINER_PREFIX` / `BACKEND_NETWORK`                                                                       | `convex-orchestrator-deployment-` / `convex-orchestrator_default`                  | Container name prefix + docker network spawned backends join.                                                                         |
| `ADMIN_EMAILS` / `REGISTRATION_MODE`                                                                                 | `admin@example.com` / `allowlist`                                                  | First admin + signup policy.                                                                                                          |
| `BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION` / `BETTER_AUTH_SMTP_URL` / `BETTER_AUTH_SMTP_FROM`                          | `0` / unset / `no-reply@orchestrator`                                              | Optional SMTP for password reset / verification links.                                                                                |
| `ORCHESTRATOR_IMAGE` / `DASHBOARD_ORCHESTRATOR_IMAGE` / `DASHBOARD_IMAGE`                                            | `ghcr.io/defy-works/convex-{orchestrator,dashboard-orchestrator,dashboard}:latest` | Pin to a specific image tag.                                                                                                          |
| `RUST_LOG`                                                                                                           | `orchestrator=info,tower_http=warn`                                                | Orchestrator log filter.                                                                                                              |

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
PUBLIC_ORCHESTRATOR_URL=http://localhost:8050 npm run dev
```

Open `http://localhost:6792`. Sign in using the `--bootstrap-token` value you
passed to the orchestrator on first start; you'll land on the team list, then
pick or create a project, then pick or provision a deployment. The deployment
view embeds `dashboard-self-hosted` (running on `:6791`) for the per-deployment
data/functions/logs/etc. UI.
