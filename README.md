<p align="center">
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://static-http.s3.amazonaws.com/logo/convex-logo-light.svg" width="600">
  <source media="(prefers-color-scheme: light)" srcset="https://static-http.s3.amazonaws.com/logo/convex-logo.svg" width="600">
  <img alt="Convex logo" src="https://static-http.s3.amazonaws.com/logo/convex-logo.svg" width="600">
</picture>
</p>

[Convex](https://convex.dev) is the open-source reactive database designed to
make life easy for web app developers, whether human or LLM. Fetch data and
perform business logic with strong consistency by writing pure TypeScript.

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
infrastructure. Many small applications and side-projects can operate entirely
on the free tier with zero cost and zero maintenance.

Development of the Convex backend is led by the Convex team. We
[welcome bug fixes](./CONTRIBUTING.md) and
[love receiving feedback](https://discord.gg/convex). We keep this repository
synced with any internal development work within a handful of days.

## Self Hosting

You can also opt to self-host Convex. The self-hosted product includes most
features of the cloud product, including the dashboard and CLI. Self-hosted
Convex works well with a variety of tools including Neon, Fly.io, Vercel,
Netlify, RDS, Sqlite, Postgres, and more. There is a
[self-hosting guide](./self-hosted/SELFHOSTING.md) available with instructions.
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

## Building from source

See [BUILD.md](./BUILD.md).

## Disclaimers

- If you choose to self-host, we recommend following the self-hosting guide. If
  you are instead building from source, make sure to change your instance secret
  and admin key from the defaults in the repo.
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
