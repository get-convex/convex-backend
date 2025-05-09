---
title: "CLI"
sidebar_position: 110
slug: "cli"
---

The Convex command-line interface (CLI) is your interface for managing Convex
projects and Convex functions.

To install the CLI, run:

```sh
npm install convex
```

You can view the full list of commands with:

```sh
npx convex
```

## Configure

### Create a new project

The first time you run

```sh
npx convex dev
```

it will ask you to log in your device and create a new Convex project. It will
then create:

1. The `convex/` directory: This is the home for your query and mutation
   functions.
2. `.env.local` with `CONVEX_DEPLOYMENT` variable: This is the main
   configuration for your Convex project. It is the name of your development
   deployment.

### Recreate project configuration

Run

```sh
npx convex dev
```

in a project directory without a set `CONVEX_DEPLOYMENT` to configure a new or
existing project.

### Log out

```sh
npx convex logout
```

Remove the existing Convex credentials from your device, so subsequent commands
like `npx convex dev` can use a different Convex account.

## Develop

### Run the Convex dev server

```sh
npx convex dev
```

Watches the local filesystem. When you change a [function](/functions.mdx) or
the [schema](/database/schemas.mdx), the new versions are pushed to your dev
deployment and the [generated types](/generated-api/) in `convex/_generated` are
updated. By default, logs from your dev deployment are displayed in the
terminal.

It's also possible to
[run a Convex deployment locally](/cli/local-deployments-for-dev.mdx) for
development.

### Open the dashboard

```sh
npx convex dashboard
```

Open the [Convex dashboard](./dashboard).

### Open the docs

```sh
npx convex docs
```

Get back to these docs!

### Run Convex functions

```sh
npx convex run <functionName> [args]
```

Run a public or internal Convex query, mutation, or action on your development
deployment.

Arguments are specified as a JSON object.

```sh
npx convex run messages:send '{"body": "hello", "author": "me"}'
```

Add `--watch` to live update the results of a query. Add `--push` to push local
code to the deployment before running the function.

Use `--prod` to run functions in the production deployment for a project.

### Tail deployment logs

You can choose how to pipe logs from your dev deployment to your console:

```sh
# Show all logs continuously
npx convex dev --tail-logs always

# Pause logs during deploys to see sync issues (default)
npx convex dev

# Don't display logs while developing
npx convex dev --tail-logs disable

# Tail logs without deploying
npx convex logs
```

Use `--prod` with `npx convex logs` to tail the prod deployment logs instead.

### Import data from a file

```sh
npx convex import --table <tableName> <path>
npx convex import <path>.zip
```

See description and use-cases:
[data import](/database/import-export/import.mdx).

### Export data to a file

```sh
npx convex export --path <directoryPath>
npx convex export --path <filePath>.zip
npx convex export --include-file-storage --path <path>
```

See description and use-cases:
[data export](/database/import-export/export.mdx).

### Display data from tables

```sh
npx convex data  # lists tables
npx convex data <table>
```

Display a simple view of the
[dashboard data page](/dashboard/deployments/data.md) in the command line.

The command supports `--limit` and `--order` flags to change data displayed. For
more complex filters, use the dashboard data page or write a
[query](/database/reading-data/reading-data.mdx).

The `npx convex data <table>` command works with
[system tables](/database/advanced/system-tables.mdx), such as `_storage`, in
addition to your own tables.

### Read and write environment variables

```sh
npx convex env list
npx convex env get <name>
npx convex env set <name> <value>
npx convex env remove <name>
```

See and update the deployment environment variables which you can otherwise
manage on the dashboard
[environment variables settings page](/dashboard/deployments/settings.md#environment-variables).

## Deploy

### Deploy Convex functions to production

```sh
npx convex deploy
```

The target deployment to push to is determined like this:

1. If the `CONVEX_DEPLOY_KEY` environment variable is set (typical in CI), then
   it is the deployment associated with that key.
2. If the `CONVEX_DEPLOYMENT` environment variable is set (typical during local
   development), then the target deployment is the production deployment of the
   project that the deployment specified by `CONVEX_DEPLOYMENT` belongs to. This
   allows you to deploy to your prod deployment while developing against your
   dev deployment.

This command will:

1. Run a command if specified with `--cmd`. The command will have CONVEX_URL (or
   similar) environment variable available:
   ```sh
   npx convex deploy --cmd "npm run build"
   ```
   You can customize the URL environment variable name with
   `--cmd-url-env-var-name`:
   ```sh
   npx convex deploy --cmd 'npm run build' --cmd-url-env-var-name CUSTOM_CONVEX_URL
   ```
1. Typecheck your Convex functions.
1. Regenerate the [generated code](/generated-api/) in the `convex/_generated`
   directory.
1. Bundle your Convex functions and their dependencies.
1. Push your functions, [indexes](/database/reading-data/indexes/indexes.md),
   and [schema](/database/schemas.mdx) to production.

Once this command succeeds the new functions will be available immediately.

### Deploy Convex functions to a [preview deployment](/production/hosting/preview-deployments.mdx)

```sh
npx convex deploy
```

When run with the `CONVEX_DEPLOY_KEY` environment variable containing a Preview
Deploy Key, this command will:

1. Create a deployment with the specified name. `npx convex deploy` will infer
   the Git branch name for Vercel, Netlify, GitHub, and GitLab environments, but
   the `--preview-create` option can be used to customize the name associated
   with the newly created deployment.
   ```
   npx convex deploy --preview-create my-branch-name
   ```
1. Run a command if specified with `--cmd`. The command will have CONVEX_URL (or
   similar) environment variable available:

   ```sh
   npx convex deploy --cmd "npm run build"
   ```

   You can customize the URL environment variable name with
   `--cmd-url-env-var-name`:

   ```sh
   npx convex deploy --cmd 'npm run build' --cmd-url-env-var-name CUSTOM_CONVEX_URL
   ```

1. Typecheck your Convex functions.
1. Regenerate the [generated code](/generated-api/) in the `convex/_generated`
   directory.
1. Bundle your Convex functions and their dependencies.
1. Push your functions, [indexes](/database/reading-data/indexes/indexes.md),
   and [schema](/database/schemas.mdx) to the deployment.
1. Run a function specified by `--preview-run` (similar to the `--run` option
   for `npx convex dev`).

   ```sh
   npx convex deploy --preview-run myFunction
   ```

See the [Vercel](/production/hosting/vercel.mdx#preview-deployments) or
[Netlify](/production/hosting/netlify.mdx#deploy-previews) hosting guide for
setting up frontend and backend previews together.

### Update generated code

```sh
npx convex codegen
```

Update the [generated code](/generated-api/) in `convex/_generated` without
pushing. This can be useful for orchestrating build steps in CI.
