<p align="center">
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://static-http.s3.amazonaws.com/logo/convex-logo-light.svg" width="600">
  <source media="(prefers-color-scheme: light)" srcset="https://static-http.s3.amazonaws.com/logo/convex-logo.svg" width="600">
  <img alt="Convex logo" src="https://static-http.s3.amazonaws.com/logo/convex-logo.svg" width="600">
</picture>
</p>

If you're new to Convex we recommend starting with the
[onboarding tutorial](https://docs.convex.dev/tutorial/) to familiarize yourself
with the Convex development experience.

If you're in this README, you're interested in self-hosting
[Convex](https://www.convex.dev) on your own infrastructure or a managed hosting
provider. Support is available on the
[Convex Discord](https://discord.gg/convex) in the `#self-hosted` channel.

If you don't specifically want to self-host, head over to
[the Convex docs](https://docs.convex.dev/) to use the cloud-hosted product.
Cloud-hosted Convex includes a generous free tier and provides a seamless,
reliable, cost-effective platform that allows you to focus on building your
application without worrying about infrastructure.

Self-hosting Convex requires deploying three services:

1. The Convex backend
1. The Convex dashboard
1. Your frontend app, which you can either host yourself or on a managed service
   like Netlify or Vercel.

# Self-hosting Convex

By default the Convex backend will store all state in a local SQLite database.
We recommend starting with this basic configuration and then moving the
container to a hosting provider or pointing the backend to a separate SQL
database for a production-ready configuration as needed.

## Docker configuration

First fetch the
[`docker-compose` file](https://github.com/get-convex/convex-backend/tree/main/self-hosted/docker/docker-compose.yml)
file then start the backend and dashboard via:

```
docker compose pull
docker compose up
```

Once the backend is running you can use it to generate admin keys for the
dashboard/CLI:

```sh
docker exec convex-local-backend ./generate_admin_key.sh
```

Visit the dashboard at `http://localhost:6791`. The backend listens on
`http://127.0.0.1:3210`. The backend's http actions are available at
`http://127.0.0.1:3211`.

In your Convex project, add your url and admin key to a `.env.local` file (which
should not be committed to source control):

```sh
CONVEX_SELF_HOSTED_URL='http://127.0.0.1:3210'
CONVEX_SELF_HOSTED_ADMIN_KEY='<your admin key>'
```

Now you can run commands in your Convex project, to push code, run queries,
import data, etc. To use these commands, you'll need the latest version of
Convex.

```sh
npm install convex@latest
```

Now you can push code, run queries, import data, etc.

```sh
npx convex dev
npx convex --help  # see all available commands
```

By default, the backend will store its data in a volume managed by Docker. Note
that you'll need to set up persistent storage on whatever cloud hosting platform
you choose to run the Docker container on (e.g. AWS EBS). By default the
database is stored locally in SQLite but you may also point it to a SQL database
either locally or on a cloud service of your choice following
[these instructions](#running-the-database-on-postgres).

You should now be able to use the self-hosted backend. Read on for alternative
hosting options for production workloads.

## Backend hosting on Fly.io

You can run the Convex backend on a hosting provider of your choice. We include
`fly.toml` files to make it easy to deploy your backend to
[Fly.io](https://fly.io/). See out dedicated [Fly instructions](./fly/README.md)
to get started.

## Running the database on Postgres

The Convex backend is designed to work well with SQLite or Postgres. If you're
running a production workload that requires guaranteed uptime it's likely you
want to use a managed Postgres service. We've included instructions below for
connecting to a Postgres database hosted on [Neon](https://neon.tech).

Use `npx convex export` to export your data before moving from one database
provider to another.

**It's very important your backend is hosted in the same region and as close as
possible to your database!** Any additional latency between backend and database
will negatively impact query performance.

### Connecting to Postgres on Neon

Copy the connection string from the Neon dashboard and create the database.

```sh
export DATABASE_CONNECTION='<connection string>'
psql $DATABASE_CONNECTION -c "CREATE DATABASE convex_self_hosted"
```

You can use the `DATABASE_URL` environment variable to instruct the backend to
connect to a certain database. This URL is the connection string without the db
name and query params. e.g., for Neon it should end in `neon.tech`:

```sh
export DATABASE_URL=$(echo $DATABASE_CONNECTION | sed -E 's/\/[^/]+(\?.*)?$//')
```

If you're running the backend on a platform like [Fly](https://fly.io), register
this environment variable in the hosting environment, e.g.,:

```sh
fly secrets set DATABASE_URL=$DATABASE_URL
```

otherwise if you're running the backend locally you can restart it to pick up
this environment variable.

Check that the database is connected to your self-hosted convex backend. There
should be a line like `Connected to Postgres` in the logs. Note that you'll have
to redeploy any existing Convex functions to the new database with
`npx convex deploy`.

## Optional configurations

- The cloud-hosted product automatically redacts logs to prevent any leaking of
  PII. If you would like to also redact log information in your self-hosted
  deployment, set the `REDACT_LOGS_TO_CLIENT` environment variable to `true`.
- Self-hosted builds contain a beacon to help Convex understand usage of the
  product. The information collected is anonymous and minimal, containing a
  random identifier plus the versions plus the versions of the backend in use.
  You may opt out of the beacon by setting the environment variable
  `DISABLE_BEACON` to `true`.

# Deploying your frontend app

The Convex backend runs all database and compute functions but it doesn't host
your actual web app. If you're hosting your website on a provider like Netlify
or Vercel using our
[production hosting instructions](https://docs.convex.dev/production/hosting/)
be sure to swap out the environment variables in those instructions for the
`SELF_HOSTED` equivalents.

e.g., instead of setting `CONVEX_DEPLOY_KEY`, you'll need to set
`CONVEX_SELF_HOSTED_URL` to the url where your Convex backend is hosted and
`CONVEX_SELF_HOSTED_ADMIN_KEY` to the admin key you generated with the
`generate_admin_key.sh` script.

## Convex Auth

If you're using Convex Auth, follow the
[manual instructions](https://labs.convex.dev/auth/setup/manual) to set up. The
CLI does not support self-hosted deployments yet.

# Software upgrades

In order to safely migrate to a new version of self-hosted, there are two
options.

## Option 1: Export/Import your database

The easiest migration path is just to export your database state and reimport it
after upgrading the backend code.

1. Take down external traffic to your backend.
2. Export your database with `npx convex export`.
3. Save your environment variables with `npx convex env list` (or via
   dashboard).
4. Upgrade the backend docker image.
5. Import from your backup with `npx convex import --replace-all`.
6. Bring back your environment variables with `npx convex env set` (or via
   dashboard)
7. Bring back external traffic to your backend.

Given that exports/imports can be expensive if you have a lot of data, this can
incur downtime. You can get a sense of how much downtime by running a test
export while your self-hosted instance is up. For smaller instances, this may be
quick and easy.

However to safely avoid losing data, it's important that the final export is
done after load is stopped from your instance, since exports are taken at a
snapshot in time.

## Option 2: Upgrade in-place

If you want to avoid downtime, you can upgrade in-place. This is a more manual
process so proceed careful and feel free to reach out for guidance.

You will need to upgrade through each intermediate binary revision specified via
`git log crates/model/src/migrations.rs`.

Each upgrade will incur a small amount of downtime, but the underlying database
will be upgraded in-place while your app still functions. You need to allow the
backend to run at each intermediate revision until it is ready.

Look for loglines like this - and follow those instructions to complete the
in-place upgrade. Each migration will let you know which logline to wait for to
determine that the in-place upgrade is complete.

```
Executing Migration 114/115. MigrationComplete(115)
```

# Limitations

Self-hosted Convex supports all the free-tier features of the cloud-hosted
product. The cloud-hosted product is optimized for scale.

# Questions and contributions

- Join our [Discord community](https://discord.gg/convex) for help and
  discussions. The `#self-hosted` channel is the best place to go for questions
  about self-hosting.

- Report issues when building and using the open source Convex backend through
  [GitHub Issues](https://github.com/get-convex/convex-backend/issues)

- We
  [welcome bug fixes](https://github.com/get-convex/convex-backend/blob/main/crates/convex/CONTRIBUTING.md)
  and love receiving feedback. We keep this repository synced with any internal
  development work within a handful of days.
