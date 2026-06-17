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

Set up routing to forward requests from your domain to the host ports exposed by
the stack — either via the bundled Traefik (`--profile tls`, see below) or your
own reverse proxy:

- `https://convex.my-domain.com` forwards to `http://localhost:6793` — the
  platform UI (sign in, teams, projects, deployments).
- `https://api.convex.my-domain.com` forwards to `http://localhost:8050` — the
  orchestrator API.
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

# Hostnames. The compose derives all browser-facing URLs as
# https://${HOST}, and Traefik (if enabled) uses these for routing.
DASHBOARD_HOST='convex.my-domain.com'
ORCHESTRATOR_HOST='api.convex.my-domain.com'
ROUTER_HOST='convex.my-domain.com'
ROUTER_PUBLIC_PORT='443'

# Optional overrides — only needed if you want a non-https scheme or
# a port-suffixed URL (e.g. local dev with traefik off):
# PUBLIC_DASHBOARD_URL='https://convex.my-domain.com'
# PUBLIC_ORCHESTRATOR_URL='https://api.convex.my-domain.com'
# PUBLIC_ORIGIN='https://convex.my-domain.com'

# Optional display labels.
# ORCHESTRATOR_REGION_NAME appears in deployment status cards.
# DEFAULT_TEAM_NAME is used when the `self-hosted` team is auto-created.
ORCHESTRATOR_REGION_NAME='Self-Hosted'
DEFAULT_TEAM_NAME='Self-Hosted'

# First admin email + signup policy. `allowlist` = only ADMIN_EMAILS can
# self-register; `open` = anyone; `invite-only` = admins plus invite links.
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

### TLS with Let's Encrypt (optional, opt-in via `--profile tls`)

The compose ships a Traefik service under the `tls` profile that terminates
80/443 on the host, auto-issues Let's Encrypt certs, and forwards to the
internal services. With it enabled, you don't run any other reverse proxy.

It uses the **DNS-01 challenge** (one wildcard cert for `*.${ROUTER_HOST}` plus
individual certs for the dashboard and orchestrator API hosts), which is
rate-limit-free no matter how many deployments you spawn. The trade-off is that
you need an API token from your DNS provider so Traefik can prove control over
your domain. The default provider is **Cloudflare**; the patterns for other
providers are identical, only the env variable names change.

#### Cloudflare setup

1. Log into the Cloudflare dashboard for the domain that hosts your
   `defy.works`-equivalent (e.g. `my-domain.com`).
2. Profile → API Tokens → Create Token → Custom token with the permissions:
   - `Zone:DNS:Edit` for the target zone
   - `Zone:Zone:Read` for the target zone
3. Copy the token. Add to `.env`:

```sh
LETSENCRYPT_EMAIL='you@my-domain.com'   # only used by LE for renewal warnings
DNS_PROVIDER='cloudflare'               # already the default; can omit
CF_DNS_API_TOKEN='<the token>'
```

(`DASHBOARD_HOST`, `ORCHESTRATOR_HOST`, and `ROUTER_HOST` are already set from
the previous step.)

DNS: point A/AAAA records for **all three hosts** at the VPS, plus a wildcard
for spawned deployments:

```
convex.my-domain.com         A   <vps-ip>
api.convex.my-domain.com     A   <vps-ip>
*.convex.my-domain.com       A   <vps-ip>
```

Bring the stack up with the `tls` profile:

```sh
docker compose -f docker-compose.orchestrator.yml --profile tls up -d
```

Traefik will pick up the labels on the existing services, request the certs from
Let's Encrypt via Cloudflare DNS, and start routing. First issuance takes
~10–30s; watch progress with `docker compose logs -f traefik`.

#### Other DNS providers

Set `DNS_PROVIDER` to your provider's slug from the
[Traefik DNS-01 provider list](https://doc.traefik.io/traefik/https/acme/#providers)
and pass through that provider's env vars. The compose already pipes through the
common ones — Cloudflare, Route 53, DigitalOcean, Gandi v5, Hetzner, Namecheap —
and any others can be added to the `traefik` service's `environment:` block.

| Provider     | `DNS_PROVIDER` slug | Env vars                                                   |
| ------------ | ------------------- | ---------------------------------------------------------- |
| Cloudflare   | `cloudflare`        | `CF_DNS_API_TOKEN`                                         |
| AWS Route 53 | `route53`           | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION` |
| DigitalOcean | `digitalocean`      | `DO_AUTH_TOKEN`                                            |
| Gandi v5     | `gandiv5`           | `GANDIV5_API_KEY`                                          |
| Hetzner      | `hetzner`           | `HETZNER_API_KEY`                                          |
| Namecheap    | `namecheap`         | `NAMECHEAP_API_USER`, `NAMECHEAP_API_KEY`                  |

#### Local dev / VPS without TLS

When you omit `--profile tls`, Traefik doesn't start and the host-port bindings
(`8050`, `6793`, etc., bound to `127.0.0.1` by default) are how you reach the
services. Set `BIND_ADDR=0.0.0.0` if you want raw exposure on the VPS public IP
— but at that point you should use the `tls` profile and have proper certs
instead.

### Environment reference

| Var                                                                                                                  | Default                                                                            | Purpose                                                                                                                               |
| -------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `BOOTSTRAP_TOKEN`                                                                                                    | `orchestrator-bootstrap-token-change-me`                                           | Initial admin PAT. Change before exposing the stack.                                                                                  |
| `SERVICE_KEY`                                                                                                        | `orchestrator-service-key-change-me`                                               | Shared secret between orchestrator and dashboard for `/api/internal/*`.                                                               |
| `BETTER_AUTH_SECRET`                                                                                                 | `better-auth-secret-change-me`                                                     | Session signing key for the dashboard auth.                                                                                           |
| `PUBLIC_ORIGIN`                                                                                                      | `http://localhost`                                                                 | Stamped into URLs the orchestrator returns to clients.                                                                                |
| `PUBLIC_DASHBOARD_URL`                                                                                               | `http://localhost:6793`                                                            | Browser URL of the platform UI; also `BETTER_AUTH_URL` and the inner dashboard's `TRUSTED_PARENT_ORIGINS`.                            |
| `PUBLIC_ORCHESTRATOR_URL`                                                                                            | `http://localhost:8050`                                                            | Browser-facing orchestrator URL. Read by the dashboard server at request time and injected into the page; no image rebuild required.  |
| `ORCHESTRATOR_REGION_NAME` / `PUBLIC_ORCHESTRATOR_REGION_NAME`                                                       | `Self-Hosted`                                                                      | Display label for orchestrated deployments' region in the dashboard status card.                                                      |
| `DEFAULT_TEAM_NAME`                                                                                                  | `Self-Hosted`                                                                      | Human-facing name for the auto-created default team. The stable slug remains `self-hosted`. Existing teams are not renamed.           |
| `ROUTER_HOST`                                                                                                        | `localhost`                                                                        | Suffix host for spawned-deployment subdomains.                                                                                        |
| `DASHBOARD_HOST` / `ORCHESTRATOR_HOST`                                                                               | unset (sentinels)                                                                  | Hostnames for Traefik `Host(\`...\`)`matchers. Required when using`--profile tls`.                                                    |
| `LETSENCRYPT_EMAIL`                                                                                                  | unset                                                                              | Email registered with Let's Encrypt (renewal warnings only). Required when using `--profile tls`.                                     |
| `DNS_PROVIDER`                                                                                                       | `cloudflare`                                                                       | Traefik DNS-01 challenge provider slug. See `--profile tls` setup above.                                                              |
| `CF_DNS_API_TOKEN` / `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` / `AWS_REGION` / `DO_AUTH_TOKEN` / others         | unset                                                                              | DNS provider creds for the DNS-01 challenge.                                                                                          |
| `BIND_ADDR`                                                                                                          | `127.0.0.1`                                                                        | Host interface that the raw service ports bind to. Set to `0.0.0.0` to expose without Traefik (not recommended on a public VPS).      |
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
  --bootstrap-token "$(openssl rand -hex 32)" \
  --default-team-name "Self-Hosted"
```

The orchestrator runs its schema migrations automatically on startup. For
PlanetScale (and any provider that requires TLS), include `sslmode=require` in
the connection URL — the orchestrator reads it and wraps the connection with
rustls.

The orchestrator listens on `0.0.0.0:8050` (BigBrain's historical port). On
first start with `--bootstrap-token` set, it creates a synthetic bootstrap
member, auto-creates the default team with slug `self-hosted`, and registers the
bootstrap token as a Personal Access Token for that member. The default team's
display name comes from `--default-team-name` /
`CONVEX_ORCHESTRATOR_DEFAULT_TEAM_NAME` and defaults to `Self-Hosted`; existing
teams keep their current name.

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
