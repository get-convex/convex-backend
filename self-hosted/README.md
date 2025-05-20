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

First download the
[`docker-compose.yml` file](https://github.com/get-convex/convex-backend/tree/main/self-hosted/docker/docker-compose.yml).
Then, to start the backend and dashboard:

```sh
docker compose up
```

Once the backend is running you can use it to generate admin keys for the
dashboard/CLI:

```sh
docker compose exec backend ./generate_admin_key.sh
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
[these instructions](#running-the-database-on-postgres-or-mysql). You can also
configure the backend to use S3 storage for exports, snapshots, modules, files,
and search indexes following [these instructions](#using-s3-storage).

You should now be able to use the self-hosted backend. Read on for alternative
hosting options for production workloads.

## Running the binary directly

<details>
<summary>Getting the binary</summary>

You can either [build from source](../README.md) or use the precompiled
binaries. You can download the latest precompiled binary release from
[Releases](https://github.com/get-convex/convex-backend/releases). If your
platform is not supported, leave us a GitHub issue. In the meantime, you can
build from source.

_Note: On MacOS you might need to hold the `option` key and double click the
binary file in Finder once, to circumvent the
[Gatekeeper](https://support.apple.com/en-us/102445) warning._

</details>

<details>
<summary>Generate a new instance secret</summary>

Instance secret is the secret to the backend. Keep very safe and only accessible
from the backend itself. Generate a new random instance secret with

```sh
cargo run -p keybroker --bin generate_secret
```

It will look like this:
`4361726e697461732c206c69746572616c6c79206d65616e696e6720226c6974`

</details>

<details>
<summary>Generate a new admin key</summary>

With the instance name and instance secret, generate an admin key. Admin key is
required to push code to the backend and take other administrator operations.

```sh
cargo run -p keybroker --bin generate_key -- convex-self-hosted 4361726e697461732c206c69746572616c6c79206d65616e696e6720226c6974
```

It will look like
`convex-self-hosted|01c046ab1512d9306a6abda3eedec5dfe862f1fe0f66a5aee774fb9ae3fda87706facaf682b9d4f9209a05e038cbd6e9b8`

</details>

<details>
<summary>Run your backend instance</summary>

Adjust the path based on where you downloaded the binary to or add it to your
`PATH`. The backend will store its database in the current-working-directory
(not where the binary file lives).

Use the instance name and instance secret to start your backend.

```sh
./convex-local-backend --instance-name convex-self-hosted --instance-secret 4361726e697461732c206c69746572616c6c79206d65616e696e6720226c6974
```

To run with Postgres, add `--db postgres-v5 <connection string>` to the command,
being sure to strip out the database name and query parameters. See
[Postgres instructions](#connecting-to-postgres-on-neon). To run with MySQL, add
`--db mysql-v5 <connection string>` to the command and similarly strip out the
database name and query parameters.

You can run `./convex-local-backend --help` to see other options for things like
changing ports, convex origin url, convex site url, local storage directories
and other configuration.

</details>

## Backend hosting on Fly.io

You can run the Convex backend on a hosting provider of your choice. We include
`fly.toml` files to make it easy to deploy your backend to
[Fly.io](https://fly.io/). See our dedicated [Fly instructions](./fly/README.md)
to get started.

## Backend hosting on Railway.com [Community maintained]

You can run the Convex backend on a hosting provider of your choice. We include
ready made template to make it easy to deploy your backend to
[Railway.com](https://railway.com/). See our dedicated
[Railway instructions](./railway/README.md) to get started.

## Backend hosting on your own infrastructure

It's possible to run Convex on your own servers, with your own routing.

Download the
[`docker-compose.yml` file](https://github.com/get-convex/convex-backend/tree/main/self-hosted/docker/docker-compose.yml)
onto the server you want to run Convex on.

```sh
curl -O https://raw.githubusercontent.com/get-convex/convex-backend/main/self-hosted/docker/docker-compose.yml
```

Your Convex backend will be running on this server at port 3210, with HTTP
actions exposed at port 3211, and the dashboard running on port 6791.

Set up routing to forward requests from your domain to these ports. For example:

- `https://api.my-domain.com` forwards to `http://localhost:3210`
- `https://my-domain.com` forwards to `http://localhost:3211`
- `https://dashboard.my-domain.com` forwards to `http://localhost:6791`

In a `.env` file beside the `docker-compose.yml` file, set the following
environment variables:

```sh
# URL of the Convex API as accessed by the client/frontend.
CONVEX_CLOUD_ORIGIN='https://api.my-domain.com'
# URL of Convex HTTP actions as accessed by the client/frontend.
CONVEX_SITE_ORIGIN='https://my-domain.com'
# URL of the Convex API as accessed by the dashboard (browser).
NEXT_PUBLIC_DEPLOYMENT_URL='https://api.my-domain.com'
```

On the server, start the backend with:

```sh
docker compose up
```

Get an admin key with:

```sh
docker compose exec backend ./generate_admin_key.sh
```

Go to the dashboard at `https://dashboard.my-domain.com` and use the admin key
to authenticate.

In your Convex project (on your local machine, probably not on the hosting
server), add the url and admin key to a `.env.local` file (which should not be
committed to source control):

```sh
CONVEX_SELF_HOSTED_URL='https://api.my-domain.com'
CONVEX_SELF_HOSTED_ADMIN_KEY='<your admin key>'
```

Now you can run commands in your Convex project, to push code, run queries,
import data, etc.

```sh
npx convex dev
```

## Running the database on Postgres or MySQL

The Convex backend is designed to work well with SQLite, Postgres, or MySQL. By
default, the docker image uses SQLite. If you're running a production workload
that requires guaranteed uptime it's likely you want to use a managed Postgres
or MySQL service. We've included instructions below for connecting to a Postgres
database hosted on [Neon](https://neon.tech) or a MySQL (Vitess) database hosted
on [PlanetScale](https://planetscale.com). We've tested that the Convex backend
works with Postgres v17 and MySQL v8, but it's possible it works with other
versions.

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

You can use the `POSTGRES_URL` environment variable to instruct the backend to
connect to a certain database. This URL is the connection string without the db
name and query params. e.g., for Neon it should end in `neon.tech`:

```sh
export POSTGRES_URL=$(echo $DATABASE_CONNECTION | sed -E 's/\/[^/]+(\?.*)?$//')
```

If you're running the backend on a platform like [Fly](https://fly.io), register
this environment variable in the hosting environment, e.g.,:

```sh
fly secrets set POSTGRES_URL=$POSTGRES_URL
```

otherwise if you're running the backend locally you can restart it to pick up
this environment variable.

Check that the database is connected to your self-hosted convex backend. There
should be a line like `Connected to Postgres` in the logs. Note that you'll have
to redeploy any existing Convex functions to the new database with
`npx convex deploy`.

### Connecting to Postgres locally

Create a database called `convex_self_hosted` in your Postgres instance.

```sh
psql postgres -c "CREATE DATABASE convex_self_hosted"
```

Set the `POSTGRES_URL` environment variable to your Postgres connection string
and disable SSL. Do not include the database name in `POSTGRES_URL`.

```sh
export POSTGRES_URL='postgresql://<your-username>@host.docker.internal:5432'
export DO_NOT_REQUIRE_SSL=1
docker compose up
```

### Running MySQL locally

```sh
mysql -e "CREATE DATABASE convex_self_hosted;"
export MYSQL_URL=mysql://<your-username>@host.docker.internal:3306
export DO_NOT_REQUIRE_SSL=1
docker compose up
```

### Running MySQL on PlanetScale

Set up a database on [PlanetScale](https://planetscale.com/). Be sure to name it
`convex_self_hosted`! Do not include the database name in `MYSQL_URL`.

```sh
export MYSQL_URL=mysql://<your-username>:<your-password>@aws.connect.psdb.cloud
docker compose up
```

## Using S3 Storage

By default, the backend stores file data on the filesystem within the docker
container. To instead run the backend with S3 storage, set up the following
buckets and environment variables.

```sh
export AWS_REGION="your-region"
export AWS_ACCESS_KEY_ID="your-access-key-id"
export AWS_SECRET_ACCESS_KEY="your-secret-access-key"
export S3_STORAGE_EXPORTS_BUCKET="convex-snapshot-exports"
export S3_STORAGE_SNAPSHOT_IMPORTS_BUCKET="convex-snapshot-imports"
export S3_STORAGE_MODULES_BUCKET="convex-modules"
export S3_STORAGE_FILES_BUCKET="convex-user-files"
export S3_STORAGE_SEARCH_BUCKET="convex-search-indexes"
```

Optionally set the `S3_ENDPOINT_URL` environment variable. This is required for
using [R2](https://www.cloudflare.com/developer-platform/products/r2/) or some
other drop-in replacement compatible with the AWS S3 API.

Then run the backend!

## Migrating storage providers

If you are switching between local storage and S3 storage (or vice versa),
you'll need to run a snapshot export and import to migrate your data.

Run:

```sh
npx convex export --path <path-to-export-file>
```

Then set up a fresh backend with the new storage provider and import the data:

```sh
npx convex import --replace-all <path-to-export-file>
```

## Optional configurations

- The cloud-hosted product automatically redacts logs to prevent any leaking of
  PII. If you would like to also redact log information in your self-hosted
  deployment, set the `REDACT_LOGS_TO_CLIENT` environment variable to `true`.
- Self-hosted builds contain a beacon to help Convex understand usage of the
  product. The information collected is anonymous and minimal, containing a
  random identifier plus the version of the backend in use. You may opt out of
  the beacon by setting the environment variable `DISABLE_BEACON` to `true`.

## Running the dashboard locally

From the `npm-packages/dashboard-self-hosted` directory, run:

```sh
just rush install
npm run build
NEXT_PUBLIC_DEPLOYMENT_URL="<your-backend-url>" npm run start
```

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

# Benchmarking

Check out our open-source benchmarking tool,
[LoadGenerator](../crates/load_generator/README.md), for more information on how
to benchmark and load test your Convex instance.

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
