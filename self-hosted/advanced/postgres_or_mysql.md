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

### Database Names

The database name that the Convex backend uses for persistence is equivalent to
the instance name (replacing `-` with `_`). If no instance name is set, the
Docker image defaults to `convex-self-hosted`, and the Convex backend will
connect to the database `convex_self_hosted`.

For the docker container, you can set the instance name via the `INSTANCE_NAME`
envrionment variable.

For example, using postgres:

```sh
export POSTGRES_URL='<connection string>'
export INSTANCE_NAME='your-instance-name'
psql $POSTGRES_URL -c "CREATE DATABASE your_instance_name;"
```
