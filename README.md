<p align="center">
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://github.com/get-convex/convex/assets/65152573/59aa88a1-6b65-4595-b485-011ed0aeb943#gh-dark-mode-only" width="600">
  <source media="(prefers-color-scheme: light)" srcset="https://github.com/get-convex/convex/assets/65152573/d6accb5f-e739-4397-9e4f-3911f0d16741" width="600">
  <img alt="Convex logo" src="https://github.com/get-convex/convex/assets/65152573/d6accb5f-e739-4397-9e4f-3911f0d16741" width="600">
</picture>
</p>

[Convex](https://convex.dev) is the backend-as-a-service for fullstack app
development. Thoughtfully curated components, optimized by experts.

Convex replaces your database, server functions, scheduling, storage, vector
search, etc. It includes
[a set of client libraries](https://github.com/get-convex/) that deeply
integrate with your frontend application code to provide fully consistent
caching and real-time updates out of the box. All you need to do is write your
application code.

The best way to get started with Convex is to follow the
[getting started guides in the Convex docs](https://docs.convex.dev/).

**Most Convex developers should be using the
[managed hosted product](https://www.convex.dev/plans).** The hosted product
includes a generous free tier and provides a seamless, reliable, cost-effective
platform that allows you to focus on building your application without worrying
about infrastructure.

**This repository is the wild west open source version.** It's the same code
used in the hosted Convex product but runs on a single machine and doesn't
include the scale-out services, replicated database backend, production
dashboard, or operational tooling. You are welcome to use this code to run your
application, either for local testing or in production, but you are on your own
managing it and ensuring the system is reliable and secure. In particular you
should ensure you have strong competency with regards to:

- Hosting
- Traffic routing
- Backups and replication
- Monitoring
- Upgrades
- Migration between versions

No official support is provided for the open source version of Convex but
community support is available in the `#open-source` channel in the
[Convex Discord](https://discord.gg/convex).

Development of the Convex backend is led by the Convex team and we are not
actively soliciting any major contributions from the community. We of course
[welcome bug fixes](./CONTRIBUTING.md) and
[love receiving feedback](https://discord.gg/convex). We keep this repository
synced with any internal development work within a handful of days.

## Getting started

We **strongly** recommend using the hosted version of Convex to get familiar
with the development workflow before attempting to run this version locally.

To get started, first install [Dependencies](#dependencies) and then see
[running the Convex backend](#running-the-convex-backend).

## Dependencies

You will need to first install the following dependencies if you don't already
have them on your machine:

- Cargo
  - The convex local backend is written in Rust. Cargo is the build system.
  - We recommend [rustup](https://rustup.rs/).
- Just
  - `cargo install just`
  - [`Just`](https://github.com/casey/just) is used throughout this guide for
    brevity. All of the vanilla commands can be found in the `Justfile`.
- The Rust nightly version specified in `rust-toolchain`
  - Assuming you installed Rust/Cargo with `rustup`, this will install
    automatically.
- The node version specified in `.nvmrc`
  - We recommend [nvm](https://github.com/nvm-sh/nvm#installing-and-updating).
  - `nvm use` from the root of the repo.
- Rush
  - `npm install --prefix scripts`
  - We manage the packages in a monorepo using [Rush](https://rushjs.io/).
- Convex JavaScript dependencies
  - `just rush install`

## Running the Convex Backend

```bash
just run-local-backend
```

Under the hood, this builds with Cargo.

```bash
cargo run -p local_backend --bin convex-local-backend
```

This command must be running at all times to serve the Convex backend.

## Provisioning a convex app locally

This example will go through running the backend with the included demo project.

**1. Start the backend**

```bash
# keep this running in the background
just run-local-backend
```

If this fails with an error "persisted db metadata ..." you might need to erase
the local database, in root directory run `rm convex_local_backend.sqlite3`.

**2. Run CLI commands**

We need to instruct the Convex CLI to talk to the local backend instead of the
hosted Convex platform. We can do this via the `--url` and `--admin-key` flags
to point to the localhost backend and a special local admin key. We have
provided Just recipes that automatically pass the appropriate flags to the cli,
e.g., instead of running `npx convex dev --admin-key [key] --url [url]` you can
just run:

```bash
just convex dev
```

To run the included demo project, you can use the following commands:

```bash
cd demo
npm i
just convex dev
```

This runs the `convex dev` code-watching service to push any application code
changes to the backend.

To run the client web application you can run the demo Vite server via:

```bash
npm run dev:client
```

Note that unlike the hosted Convex workflow, we don't want to run the
`dev:server` command since the backend is already running.

_The following CLI commands may be useful when interacting with your backend:_

- `just convex data` - Lists tables in your Convex deployment
- `just convex env` - Allows you to list/set/update/delete environment variables
- `just convex logs` - Streams out log lines to the terminal (it includes all
  successful executions if `--success` is passed in)
- `just convex import` - Allows you to import tables
- `just convex export` - Allows you to export tables

## Documentation

For full documentation visit [docs.convex.dev](https://www.docs.convex.dev).

To see how to contribute, visit [Contributing.md](./CONTRIBUTING.md).

## Community & Support

- Discord. Best for: sharing your applications, hanging out with the community,
  and help with building on Convex
- GitHub Issues. Best for: surfacing bugs and errors you encounter while
  building and using the open source Convex backend

## Disclaimers

- The Convex local backend is designed for use in local development and testing.
  Please exercise caution if attempting to use it for production usage. Convex
  can't offer support or guarantees for that experience - you're on your own
  there. If you do choose to go down that route, make sure to change your admin
  key from the defaults in the repo.
- The Convex local backend doesn't have backward compatibility guarantees for
  CLI compatibility. Once a feature is released in a CLI, the backend will
  support it from that point on, but newer CLI features may not work with older
  backends. Unreleased/beta features won't have any guarantees.
- Convex local backend does not support robust backend migrations. We'll always
  ensure things will work if you wipe your local database and start from
  scratch. Upgrading an existing local-backend to a newer version is not
  supported.

## Repository layout

- `crates/` contains Rust code

  - Main binary
    - `local_backend/` is an application server on top of the `Runtime`. This is
      the serving edge for the Convex cloud.

- `npm-packages/` contains both our public and internal TypeScript packages.
  - Internal packages
    - `udf-runtime/` sets up the user-defined functions JS environment for
      queries and mutations
    - `udf-tests/` is a collection of functions used in testing the isolate
      layer
    - `system-udfs/` contains functions used by the Convex system e.g. the CLI
- `demo/` contains a demo project that showcases the basic functionality of
  Convex
