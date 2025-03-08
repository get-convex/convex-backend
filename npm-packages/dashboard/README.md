# Convex Dashboard

This is the dashboard for Convex Cloud. If you intend to contribute to the
self-hosted dashboard, see the
[dashboard-self-hosted README](../dashboard-self-hosted/README.md). You may also
connect the self-hosted dashboard to a Convex Cloud deployment.

The following instructions are intended for Convex employees developing the
Cloud hosted dashboard.

## Running against a local `big-brain`.

### First time setup

First run `just rush install` to install dependencies.

You need environment variables set up to run the dashboard locally. First, run
`npx vercel link` to link your local instance to the Vercel project. You’ll need
to log in a Vercel account that is part of the Convex organization.

```
$ npx vercel link
Vercel CLI 30.2.3
> > No existing credentials found. Please log in:
? Log in to Vercel github
> Success! GitHub authentication complete for nicolas.ettlin@me.com
? Set up “~/Documents/convex-alt/npm-packages/dashboard”? [Y/n] y
? Which scope should contain your project? Convex
? Found project “convex-dev/dashboard”. Link to it? [Y/n] y
✅  Linked to convex-dev/dashboard (created .vercel)
```

Then, you can run this command to pull an env configuration from Vercel:

```
npm run pullEnv
```

### Start the dashboard local server

Run `just run-dash`. It will prompt you to start big brain, in which case run
`just run-dash` again in another terminal.

Log in to the dashboard with a GitHub account (which will create an account on
our development Auth0 instance).

### Reset big brain

You might need to clear the big brain database from time to time (or if you want
to reset to empty state, no login):

```bash
just bb-clean-db
```

### Create a project

Let's create a new project.

```bash
cd ../demos/tutorial
just convex-bb dev
```

Now you should be able to see your project on the dashboard.

### Developing NPM

If you make changes to any NPM packages used by the dashboard run
`just rush build -t convex` and restart the local server.

## Testing strategies

We have a few tools for testing in the dashboard. It is recommended to write
tests for new code and regressions you fix, but not required. However, be sure
to always test your changes via the Vercel deployment previews attached to your
GitHub pull requests.

### Unit tests

`npm run test` -- runs jest tests. these tests will also be run in CI

### Integration tests

## Bundle size

You can analyze the bundle size of the dashboard by running
`ANALYZE=true npm run build`.

## Storybook

We use [Storybook](https://storybook.js.org/) as a component library for
documentation the behavior and visual aspects of the dashboard design system.
Primitive components that do not depend on any Convex-related data or concepts
belong in the `src/elements` directory.

You can start Storybook using:

```bash
cd ~/src/convex/npm-packages/dashboard
npm run storybook
```

## Running the local dashboard against production

For this, we will proxy big brain via a cors proxy. Set these values in
`.env.local`. Some must be copied from
[Production vercel env vars](https://vercel.com/convex-dev/dashboard/settings/environment-variables)

```
NEXT_PUBLIC_BIG_BRAIN_URL=http://localhost:8080/https://api.convex.dev
AUTH0_CLIENT_ID=nANKpAFe4scUPxW77869QHVKYAgrPwy7
AUTH0_ISSUER_BASE_URL=https://auth.convex.dev
DISABLE_BIG_BRAIN_SSR=1

AUTH0_SECRET={copy from production env vars}
AUTH0_CLIENT_SECRET={copy from production env vars}
```

Run the CORS Anywhere proxy locally:

`npm run corsAnywhere`

Now when you `npm run dev:pure`, the dashboard will talk to production big brain
and backends.

Make sure you log out before you want to switch back, otherwise open
`http://localhost:6789/api/auth/logout` to log out if you get into a broken
state.
