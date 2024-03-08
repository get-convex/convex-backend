<p align="center">
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://github.com/get-convex/convex/assets/65152573/59aa88a1-6b65-4595-b485-011ed0aeb943#gh-dark-mode-only" width="600">
  <source media="(prefers-color-scheme: light)" srcset="https://github.com/get-convex/convex/assets/65152573/d6accb5f-e739-4397-9e4f-3911f0d16741" width="600">
  <img alt="Shows an illustrated sun in light mode and a moon with stars in dark mode." src="https://github.com/get-convex/convex/assets/65152573/d6accb5f-e739-4397-9e4f-3911f0d16741" width="600">
</picture>
</p>

[Convex](https://convex.dev) is the fullstack TypeScript development platform.
Replace your database, server functions and glue code.

[TODO: James will add some blurb here about Convex]

Tenatively, here are some of the things it will contain.

- Open core of the distributed convex cloud enviornment
- Good for simply playing with the code or potentially for running integration
  tests
- Not intended to run in production because it isn’t the full cloud
- Does not come with any warranties or support from the Convex team
- Indication that folks should go to dashboard.convex.dev for hosted offering

## Getting started

To get started, first install [Dependencies](#dependencies) and then see
[running the Convex backend](#running-the-convex-backend)

## Dependencies

- cargo
  - The convex local backend is written in rust. Cargo is the build system.
  - We recommend [rustup](https://rustup.rs/)
- just
  - `cargo install just`
  - [`Just`](https://github.com/casey/just) is used throughout this guide for
    brevity. All of the vanilla commands can be found in the `Justfile`.
- rust nightly version specified in `rust-toolchain`
  - Assuming you installed rust/cargo with `rustup`, this will install
    automatically
- node version specified in `.nvmrc`
  - We recommend [nvm](https://github.com/nvm-sh/nvm#installing-and-updating)
  - `nvm use` from the root of the repo
- [rush](https://rushjs.io/)
  - `npm install --prefix scripts`
  - We manage the packages in a monorepo using [Rush](https://rushjs.io/).
- Install JS dependencies
  - `just rush install`

## Running the Convex Backend

```bash
just run-local-backend
```

Under the hood, this builds with cargo.

```bash
cargo run -p local_backend --bin convex-local-backend
```

## Provisioning a convex app locally

This example will go through running the backend with the included tutorial
project.

**1. Start the backend**

```bash
just run-local-backend
```

If this fails with an error "persisted db metadata ..." you might need to erase
the local database, in root directory run `rm convex_local_backend.sqlite3`.

**2. Run CLI commands**

Convex local Backend works with the Convex CLI. You can use CLI commands in a
demo directory like `npm-packages/demos/tutorial` or in your own projects.

We've provided `just` recipes that set up the admin key to work with your local
backend. Note that you may have to copy the `Justfile` into your own project to
use these recipes.

For example:

```bash
just convex dev
```

_`just convex [arg]` can be replaced with
`npx convex [arg] --admin-key [key] --url [url]`. `just` is used here for
brevity._

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

- `crates/` contains rust code

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
  - `tutorial/` contains a tutorial project
