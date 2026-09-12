# Building from source

## Building the local backend

These instructions allow you to build and run the full Convex backend.

### Installing dependencies

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
- pnpm + Turborepo
  - `npm clean-install --prefix scripts`
  - We manage the packages in a pnpm workspace with Turborepo as the task
    runner.
- Convex JavaScript dependencies
  - `just install-js`

#### Building from source

Build and run the local backend from the source in this repo:

```sh
just run-local-backend
```

#### Local process lifecycle on Windows

The Convex CLI owns every local backend it starts. Current CLI and backend
versions keep the backend's standard input connected; closing that pipe asks the
backend to drain requests, stop workers, and remove its temporary files. The
pipe also closes when the CLI exits unexpectedly. On Windows, the CLI starts
current backends in a hidden, detached process group. This prevents Windows from
ending the backend with its Node parent before the backend can observe the
closed pipe.

The local backend owns each Node action executor. On Windows it assigns the
executor to a Job Object configured with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
Windows therefore terminates the executor if the backend is terminated before
Rust cleanup can run. During an ordinary shutdown, the backend stops and reaps
the executor before removing its temporary directory. Removal uses a short,
fixed retry window for transient Windows sharing violations and reports a
persistent cleanup failure instead of silently leaking the directory.

Run the focused Windows lifecycle tests after changing this boundary:

```powershell
cargo test -p node_executor force_killing_job_owner_terminates_assigned_process
cargo test -p node_executor ordinary_drop_stops_executor_and_removes_temp_dir
cargo test -p node_executor explicit_shutdown_stops_executor_and_removes_temp_dir
cargo test -p node_executor force_killing_launcher_triggers_clean_executor_shutdown
```

The first test force-terminates the process that owns a Job Object and verifies
that the assigned process does not survive. The next two exercise Drop and the
explicit application shutdown interface. The final test force-terminates a
launcher that owns the backend's stdin pipe, then verifies that the backend
closes the Node executor, named pipe, and temporary directory before exiting.

The lifecycle contract activates when the installed CLI and downloaded local
backend both contain this change. Verify activation by running a Node action,
ending `convex dev`, and confirming that its backend PID, executor PID,
`cvx-node-executor-*` pipe, and `.tmp*` executor directory are absent. To roll
back while diagnosing an unrelated regression, use the previous CLI release
with its matching backend release. A current CLI may add arguments that a
pre-lifecycle backend cannot parse. Stop the current backend cleanly and inspect
its process tree before switching versions; the older pair restores the
previous Windows teardown behavior.

### Provisioning a demo app locally

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
just install-js
cd npm-packages/demos/tutorial
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

If you're using both the local backend and the hosted cloud platform, make sure
to run `npx convex dev` or `just convex dev` before you start testing your
client. The `dev` command will take care of updating your `.env.local` file with
the correct `CONVEX_URL`.

## Building NPM packages (client, CLI, docs, dashboard, ESLint plugin…)

If you want to make changes to individual NPM packages (such as `convex`,
`@convex-dev/eslint-plugin`, the dashboard, and docs), you can install the
required dependencies and build the packages this way:

```sh
npm clean-install --prefix scripts
just install-js

# Builds the entire monorepo
just turbo run build
# You can also build individual packages and their dependencies, for example: just turbo run build --filter=docs...
```

For development, individual packages have useful commands in their
`package.json` file. For example, to run a dev server for docs, you will need to
run:

```sh
cd npm-packages/docs
just turbo run build --filter=docs^... # builds the packages docs rely on
npm run dev
```

If you need to modify the dependencies of monorepo packages, modify the right
`package.json` file, and then run `just update-js`.
