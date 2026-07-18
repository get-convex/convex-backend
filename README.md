<p align="center">
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://static.convex.dev/logo/convex-logo-light.svg" width="600">
  <source media="(prefers-color-scheme: light)" srcset="https://static.convex.dev/logo/convex-logo.svg" width="600">
  <img alt="Convex logo" src="https://static.convex.dev/logo/convex-logo.svg" width="600">
</picture>
</p>

[![Default branch: enhanced](https://img.shields.io/badge/default%20branch-enhanced-0f766e)](https://github.com/defy-works/convex-backend/tree/enhanced)
[![Upstream base: get-convex/convex-backend](https://img.shields.io/badge/upstream-get--convex%2Fconvex--backend-111827)](https://github.com/get-convex/convex-backend)

## Defy Enhanced Fork

This repository is the Defy-maintained Convex fork. The default branch is
`enhanced`, which tracks upstream Convex and carries fork-specific operational
improvements for self-hosting.

### Enhancements in this fork

- Configurable isolate worker cap via `FUNRUN_MAX_ISOLATE_WORKERS` instead of a
  hardcoded `128` worker limit.
- Self-hosted dashboard admin key management. Create, rename, and revoke admin
  keys from **Settings → Admin Keys** instead of shelling into the container for
  every key. Keys minted by `generate_admin_key.sh` (or pasted into
  `CONVEX_SELF_HOSTED_ADMIN_KEY`) auto-adopt into the list on first use, so
  every key in active use is revocable without changing your workflow. Each row
  shows the last few characters of the key so you can tell which one you're
  revoking.
- Self-hosted dashboard **Backups** tab. Trigger an immediate snapshot from the
  dashboard and download the resulting `.zip` (powered by the same backend
  endpoints `npx convex export` already uses). Restore is via
  `npx convex import --replace-all backup.zip` per the upstream docs.
- Self-hosted dashboard **Custom Domains** tab. Surfaces the live values of the
  `CONVEX_CLOUD_ORIGIN` and `CONVEX_SITE_ORIGIN` environment variables (with
  copy buttons) and points users at the self-hosting guide for changing them,
  instead of being disabled with a tooltip.
- Self-hosted deployment **Summary** card on the Health and Settings → General
  pages: "Self-Hosted" badge, backend version with an "update available" notice,
  last-deployment timestamp, and the deployment URL + HTTP Actions URL.
- Scheduled upstream sync automation so `enhanced` stays aligned with
  `get-convex/convex-backend` and `release` stays aligned with upstream release
  updates.

### Branches

- `enhanced`: default branch for this fork, where Defy patches live on top of
  upstream `main`.
- `release`: release-tracking branch used to build and publish self-hosted
  images when upstream release updates land.

### Support this fork

This fork is currently solo-maintained, and the running costs come out of my own
pocket — AWS runners for the release workflows, time spent reviewing incoming
PRs, and the occasional infra fix on top of upstream Convex. If the fork is
useful to your self-hosted setup, a coffee is appreciated and entirely optional.

[![Support on Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/defyworks)

This fork's CI runs on [RunsOn](https://runs-on.com), ephemeral GitHub Actions
runners in our own AWS account.

---

## About Convex

[Convex](https://convex.dev) is the open-source reactive database designed to
make life easy for web app developers, whether human or LLM. Fetch data and
perform business logic with strong consistency by writing pure TypeScript.

Convex provides a database, a place to write your server functions, and client
libraries. It makes it easy to build and scale dynamic live-updating apps.
[Read the docs to learn more](https://docs.convex.dev/understanding/).

Development of the Convex backend is led by the Convex team. We
[welcome small bug fixes](./CONTRIBUTING.md) and
[love receiving feedback](https://discord.gg/convex). We keep this repository
synced with any internal development work within a handful of days. Convex is a
well tested piece of software, with several well designed test frameworks
including randomized testing. Those tests are not provided as part of the open
source offering.

## Getting Started

Visit our [documentation](https://docs.convex.dev/) to learn more about Convex
and follow our getting started guides.

The easiest way to build with Convex is through our
[cloud platform](https://www.convex.dev/plans), which includes a generous free
tier and lets you focus on building your application without worrying about
infrastructure. Many small applications and side-projects can operate entirely
on the free tier with zero cost and zero maintenance.

## Self Hosting

The self-hosted product includes most features of the cloud product, including
the dashboard and CLI. Self-hosted Convex works well with a variety of tools
including Neon, Fly.io, Vercel, Netlify, RDS, Sqlite, Postgres, and more.

You can either use Docker (recommended) or a prebuilt binary to self host
Convex. Check out our [self-hosting guide](./self-hosted/README.md) for detailed
instructions. Community support for self-hosting is available in the
`#self-hosted` channel on [Discord](https://discord.gg/convex).

## Community & Support

- Join our [Discord community](https://discord.gg/convex) for help and
  discussions.
- Report issues when building and using the open source Convex backend through
  [GitHub Issues](https://github.com/get-convex/convex-backend/issues)
- By submitting pull requests, you confirm that Convex can use, modify, copy,
  and redistribute the contribution, under the terms of its choice.

## Building from source

See [BUILD.md](./BUILD.md).

## Disclaimers

- If you choose to self-host, we recommend following the self-hosting guide. If
  you are instead building from source, make sure to change your instance secret
  and admin key from the defaults in the repo.
- Convex is battle tested most thoroughly on Linux and Mac. On Windows, it has
  less experience. If you run into issues, please message us on
  [Discord](https://convex.dev/community) in the `#self-hosted` channel.
- Convex self-hosted builds contain a beacon to help Convex improve the product.
  The information is minimal and anonymous and helpful to Convex, but if you
  really want to disable it, you can set the `--disable-beacon` flag on the
  backend binary. The beacon's messages print in the log and only include
  - A random identifier for your deployment (not used elsewhere)
  - Migration version of your database
  - Git rev of the backend
  - Uptime of the backend

## Repository layout

- `crates/` contains Rust code

  - Main binary
    - `local_backend/` is an application server on top of the `Runtime`. This is
      the serving edge for the Convex cloud.

- `npm-packages/` contains both our public and internal TypeScript packages.
  - Internal packages
    - `udf-runtime/` sets up the user-defined functions JS environment for
      queries and mutations
    - `system-udfs/` contains functions used by the Convex system e.g. the CLI
