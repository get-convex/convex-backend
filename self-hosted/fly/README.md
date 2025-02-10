# Self Hosting with [Fly.io](https://fly.io/)

## Setup

Copy the
[`fly` files](https://github.com/get-convex/convex-backend/tree/main/self-hosted/fly)
to your local machine. You don't need to copy it into your project directory,
but you can. `degit` is a tool for copying files from git repositories.

```sh
npx degit get-convex/convex-backend/self-hosted/fly fly
cd fly
```

Install the `fly` CLI by following
[these instructions](https://fly.io/docs/flyctl/install/)

## Deploying the backend to Fly.io

The backend "deploy" can mean two things:

1. Deploying the Convex docker image to Fly.io.
2. Deploying your app's Convex functions to the fly machine running Convex.

Steps:

1. Deploy the backend to Fly.io.

   ```sh
   cd backend
   fly launch
   ```

   Future deployments can be done with `fly deploy`.

2. Note the URL of the app that gets printed out, which will be of the form
   `https://<app-name>.fly.dev` (no trailing slash).

   In the fly.toml file, change the env variables `CONVEX_CLOUD_ORIGIN` and
   `CONVEX_SITE_ORIGIN` to:

   ```sh
   CONVEX_CLOUD_ORIGIN="<fly-app-url>"
   CONVEX_SITE_ORIGIN="<fly-app-url>/http"
   ```

   And re-deploy to pick up the changes.

   ```sh
   fly deploy
   ```

   Copy and paste the fly url to set `NEXT_PUBLIC_DEPLOYMENT_URL` in the
   dashboard/fly.toml file.

3. Check that the backend is running.

   ```sh
   curl <fly-app-url>/instance_name
   ```

   You should see the instance name printed out, `convex-self-hosted` by
   default. Check the logs with `fly logs` if it's not working.

4. Generate an admin key.

   ```sh
   fly ssh console --app <app-name> --command "./generate_admin_key.sh"
   ```

   This admin key will be used to authorize the CLI and access the dashboard.

5. Inside your app, create a `.env.local` file with the following variables:

   ```sh
   CONVEX_SELF_HOST_URL='<fly-app-url>'
   CONVEX_SELF_HOST_ADMIN_KEY='<your-admin-key>'
   ```

6. To deploy your Convex functions to the backend, you'll use the `convex` CLI.

   In your frontend app directory, install `convex`.

   ```sh
   cd <your-frontend-app-directory>
   npm install convex@alpha
   ```

   To continuously deploy code for development:

   ```sh
   npx convex self-host dev
   ```

   This will continuously deploy your functions as you edit them. It will also
   set environment variables for your frontend, like `VITE_CONVEX_URL`. If you
   only want to deploy once, run `npx convex self-host dev --once`.

   To deploy code once, e.g. for production:

   ```sh
   npx convex self-host deploy --env-file <path to env file>
   ```

   If you don't want to ues a path, call it with the env variables set. It will
   not read any .env file by default.

   **Note:** It's up to you whether a backend is for development or production.
   There is no distinction within the instance. If you only have one backend,
   you can run `npx convex self-host dev` or `npx convex self-host deploy`
   depending on whether you want it to live-update or not.

   An extension of this is that you can have many backends for staging or
   previews. The difference will be in the environment variables.

### HTTP Actions

Note that HTTP actions run on your fly app url under the `/http` path. For
example:

- If your fly app is deployed at `https://self-hosted-backend.fly.dev`
- And you have an HTTP action named `/sendEmail`
- You would call it at `https://self-hosted-backend.fly.dev/http/sendEmail`

## Deploying the dashboard to Fly.io

The dashboard allows you to see logs, read/write data, run functions, and more.
You can run the dashboard locally (see [SELFHOSTING.md](SELFHOSTING.md)), or
also deploy it to Fly.io.

1. Go into the dashboard directory where you copied the self-hosted files.

   ```sh
   cd dashboard
   ```

2. Update `NEXT_PUBLIC_DEPLOYMENT_URL` in the dashboard/fly.toml file to the url
   of your fly-hosted backend, if you haven't already. Make sure there is no
   trailing slash.

   ```sh
   NEXT_PUBLIC_DEPLOYMENT_URL="<fly-app-url>"
   ```

3. Deploy the dashboard to Fly.io.

   ```sh
   fly launch
   ```

   You should now be able to visit the dashboard at the url output by fly.

4. Visit the dashboard and enter the admin key. To log in, it will need the
   admin key you generated earlier.

   You should see your tables, see and run functions, etc.
