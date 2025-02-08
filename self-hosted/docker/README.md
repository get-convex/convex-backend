# Self Hosting Via Docker

You'll need to have [Docker](https://docs.docker.com/desktop/) installed to run
convex in Docker.

```sh
cd self-hosted
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

Using your admin key, push code to your backend. Admin key should be kept secure
to just the developers who are administering the application on your backend.

Visit the dashboard at `http://localhost:6791`. The backend listens on
`http://127.0.0.1:3210`. The backend's http actions are available at
`http://127.0.0.1:3211`.

In your Convex project, add your url and admin key to a `.env.local` file (which
should not be committed to source control):

```sh
CONVEX_SELF_HOST_URL='http://127.0.0.1:3210'
CONVEX_SELF_HOST_ADMIN_KEY='<your deploy key>'
```

Now you can run commands in your Convex project, to push code, run queries,
import data, etc. To use these commands, you'll need the latest alpha version of
Convex.

```sh
npm install convex@alpha
```

```sh
npx convex self-host dev
npx convex self-host run <run args>
npx convex self-host import <import args>
npx convex self-host --help  # see all available commands
```

By default, the backend will store its data in a volume managed by Docker. Note
that you'll need to set up persistent storage on whatever cloud hosting platform
you choose to run the Docker container on (e.g. AWS EBS). The default database
is SQLite, but for production workloads, we recommend running Convex backed by
Postgres. Follow
[these instructions](../README.md#self-hosting-on-postgres-with-neon) to connect
to Postgres.
