# Self Hosting Via Docker

You'll need to have [Docker](https://docs.docker.com/desktop/) installed to run
convex in Docker.

Copy the
[`docker-compose` file](https://github.com/get-convex/convex-backend/tree/main/self-hosted/docker/docker-compose.yml)
to your local machine. You don't need to copy it into your project directory,
but you can. `degit` is a tool for copying files from git repositories.

```sh
npx degit get-convex/convex-backend/self-hosted/docker/docker-compose.yml docker-compose.yml
# Pull the latest docker images
docker compose pull
# Run the containers
docker compose up
```

Note: if you see permissions issues, try `docker logout ghcr.io`.

Once your Convex backend is running in Docker, you can ask it to generate admin
keys for use from the dashboard/CLI.

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
import data, etc. To use these commands, you'll need the latest alpha version of
Convex.

```sh
npm install convex@alpha
```

```sh
npx convex dev
npx convex run <run args>
npx convex import <import args>
npx convex --help  # see all available commands
```

By default, the backend will store its data in a volume managed by Docker. Note
that you'll need to set up persistent storage on whatever cloud hosting platform
you choose to run the Docker container on (e.g. AWS EBS). The default database
is SQLite, but for production workloads, we recommend running Convex backed by
Postgres. Follow
[these instructions](../README.md#self-hosting-on-postgres-with-neon) to connect
to Postgres.
