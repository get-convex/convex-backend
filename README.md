<p align="center">
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://static-http.s3.amazonaws.com/logo/convex-logo-light.svg" width="600">
  <source media="(prefers-color-scheme: light)" srcset="https://static-http.s3.amazonaws.com/logo/convex-logo.svg" width="600">
  <img alt="Convex logo" src="https://static-http.s3.amazonaws.com/logo/convex-logo.svg" width="600">
</picture>
</p>

[Convex](https://convex.dev) is the open-source reactive database for web app
developers. Fetch data and perform business logic with strong consistency by
writing pure TypeScript.

Convex replaces your database, server functions, scheduling, storage, vector
search, etc. It includes
[a set of client libraries](https://github.com/get-convex/) that deeply
integrate with your frontend application code to provide fully consistent
caching and real-time updates out of the box. All you need to do is write your
application code.

The best way to get started with Convex is to follow the
[getting started guides in the Convex docs](https://docs.convex.dev/).

The **[Convex managed hosted product](https://www.convex.dev/plans).** includes
a generous free tier and provides a seamless, reliable, cost-effective platform
that allows you to focus on building your application without worrying about
infrastructure.

Development of the Convex backend is led by the Convex team. We
[welcome bug fixes](./CONTRIBUTING.md) and
[love receiving feedback](https://discord.gg/convex). We keep this repository
synced with any internal development work within a handful of days.

## Self Hosting

You can also opt to self-host Convex. Convex works with a variety of tools
including Neon, Fly.io, RDS, sqlite, postgres, and more. There is a
[self-hosting guide](./self-hosted/SELFHOSTING.md) available with some guidance.
Community support for self-hosting is available in the `#open-source` channel in
the [Convex Discord](https://discord.gg/convex).

## Documentation

For full documentation visit [docs.convex.dev](https://docs.convex.dev).

To see how to contribute, visit [Contributing.md](./CONTRIBUTING.md).

## Community & Support

- [Discord](https://convex.dev/community). Best for: sharing your applications,
  hanging out with the community, and help with building on Convex
- GitHub Issues. Best for: surfacing bugs and errors you encounter while
  building and using the open source Convex backend

## Running on your own machine

Instead of self-hosting via docker, you can run the Convex backend locally on
your own machine.

## Using prebuilt binaries

You can download the latest precompiled binary release from
[Releases](https://github.com/get-convex/convex-backend/releases). Only Apple
x64, Apple Arm64 (Apple silicon), and Linux x64 binaries are currently available
for download.

_Note: On MacOS you might need to hold the `option` key and double click the
binary file in Finder once, to circumvent the
[Gatekeeper](https://support.apple.com/en-us/102445) warning._

Then you can run it:

```sh
./convex-local-backend
```

Adjust the path based on where you downloaded the binary to or add it to your
`PATH`. The backend will store its database in the directory where it is
executed **from** (not where the binary file lives).

## Building from source

To get started, clone this repo:

```sh
git clone https://github.com/get-convex/convex-backend.git
cd convex-backend
```

## Dependencies

You will need to first install the following dependencies if you don't already
have them on your machine:

To use the scripts set up in this repo:

- [`Just`](https://github.com/casey/just)
  - Just is used to execute scripts set up in the `Justfile`.
  - To install it see
    [Packages](https://github.com/casey/just?tab=readme-ov-file#packages), for
    example `cargo install just` or `brew install just`

To run the Convex CLI:

- [Node.js](https://nodejs.org/en)
  - Make sure you have the version specified in `.nvmrc`
  - We recommend installing Node.js via
    [nvm](https://github.com/nvm-sh/nvm#installing-and-updating).
  - Run `nvm use` from the root of the repo.

To [build the backend from source](#building-from-source):

- Cargo
  - The convex local backend is written in Rust. Cargo is the build system.
  - We recommend installing Cargo via [rustup](https://rustup.rs/).
- The Rust nightly version specified in `rust-toolchain`
  - Assuming you installed Rust/Cargo with `rustup`, this will install
    automatically.
- Rush
  - `npm clean-install --prefix scripts`
  - We manage the packages in a monorepo using [Rush](https://rushjs.io/).
- Convex JavaScript dependencies
  - `just rush install`

### Building from source

Build and run the local backend from the source in this repo:

```sh
just run-local-backend
```

Under the hood, this builds with Cargo:

```sh
cargo run -p local_backend --bin convex-local-backend
```

## Provisioning a demo app locally

This example will go through running the backend with the included demo project.

**1. Start the backend**

[Run the backend](#running-the-convex-backend)

If this fails with an error "persisted db metadata ..." you might need to erase
the local database, in root directory run `rm convex_local_backend.sqlite3`.

**2. Develop against the backend**

The Convex CLI watches for changes in the application source code and pushes the
code to backend.

To make the local backend run the included demo project, do:

```bash
cd demo
npm i
just convex dev
```

The `convex` script in `Justfile` automatically adds appropriate `--url` and
`--admin-key` flags to point the CLI to the local backend.

To run the client web application you can run the demo Vite server via:

```bash
npm run dev:frontend
```

Note that unlike the hosted Convex workflow, we don't want to run the
`dev:backend` command since `convex dev` is already running.

_The following CLI commands may be useful when interacting with your backend:_

- `just convex data` - Lists tables in your Convex deployment
- `just convex env` - Allows you to list/set/update/delete environment variables
- `just convex logs` - Streams out log lines to the terminal (it includes all
  successful executions if `--success` is passed in)
- `just convex import` - Allows you to import tables
- `just convex export` - Allows you to export tables

## Disclaimers

- If you choose to self-host, we recommend following the self-hosting guide. If
  you are going off the beaten path, make sure to change your instance secret
  and admin key from the defaults in the repo.
- Migrating to a new version of self-hosted requires carefully following the
  migration guide in our release notes.
- If you're using both the local backend and the hosted cloud platform, make
  sure to run `npx convex dev` or `just convex dev` before you start testing
  your client. The `dev` command will take care of updating your `.env.local`
  file with the correct `CONVEX_URL`.
- Convex is battle tested most thoroughly on Linux and Mac. On Windows, it has
  less experience. If you run into issues, please message us on
  [Discord](https://convex.dev/community) in the #open-source channel.
- Convex local-backend and self-host products contain a beacon to help Convex
  improve the product. The information is minimal and anonymous and helpful to
  Convex, but if you really want to disable it, you can set the
  `--disable-beacon` flag on the backend binary. The beacon's messages print in
  the log and only include
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
    - `udf-tests/` is a collection of functions used in testing the isolate
      layer
    - `system-udfs/` contains functions used by the Convex system e.g. the CLI
- `demo/` contains a demo project that showcases the basic functionality of
  Convex using React
