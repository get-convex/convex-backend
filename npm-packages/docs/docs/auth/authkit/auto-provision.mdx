---
title: "Automatic AuthKit Configuration"
sidebar_label: "Automatic Config"
sidebar_position: 20
description:
  "Configure WorkOS AuthKit integration with automatic provisioning for Convex
  deployments"
---

Convex can **create** AuthKit environments in a WorkOS account made on your
behalf. By default WorkOS gives you only two environments, but giving each
Convex dev deployment its own AuthKit environment is useful for isolating
development user data and configuration changes between multiple developers or
agents working in parallel.

The Convex CLI will **configure** AuthKit environments, regardless of whether
Convex or you created them, if the `WORKOS_CLIENT_ID` and `WORKOS_API_KEY`
environment variables are present in the build environment or the Convex
deployment. While developing locally, Convex can write environment variables to
`.env.local` to make setting up an AuthKit environment a breeze.

## Getting Started

Choose AuthKit as your authentication option in the create convex tool.

```bash
npm create convex@latest
```

These templates include a `convex.json` which cause an AuthKit environment for
this deployment to be created and configured. With just `npx convex dev` you'll
get a WorkOS environment all hooked up!

### Going to production

In the Convex dashboard settings for your production deployment, create an
AuthKit environment in the WorkOS Authentication integration under settings,
integrations. Copy these credentials to your hosting provider environment
variables (in addition to other setup, like adding a production
`CONVEX_DEPLOY_KEY`, setting the build command, and setting other
framework-specific AuthKit environment variables).

### Preview deployments

In the Convex dashboard settings for any deployment in your project, create a
new project-level AuthKit environment in the WorkOS Authentication integration
under settings, integrations. Copy these credentials to your hosting provider
environment variables (in addition to other setup, like adding a preview
`CONVEX_DEPLOY_KEY`, setting the build command, and setting other
framework-specific AuthKit environment variables).

## How it works

AuthKit provisioning and configuration is triggered by the presence of a
`convex.json` file with an `authKit` section with a property corresponding to
the type of code push: `dev`, `preview`, or `prod`.

If this section is present, an AuthKit environment may be provisioned (dev
only), local environment variables set (dev only), and configured (all code push
types).

### Finding the AuthKit environment

The CLI looks for WorkOS credentials `WORKOS_CLIENT_ID` and `WORKOS_API_KEY` in
the following order:

1. Environment variables in the build environment shell or `.env.local` file
2. Convex deployment environment variables

In remote build environments (e.g. building a project in Vercel, Netlify,
Cloudflare) if these two environment variables are not found, the build will
fail.

During local dev, credentials are next fetched from the Convex Cloud API for a
new or existing AuthKit environment. A link to this deployment in the WorkOS
dashboard can be found in the Convex dashboard under the WorkOS integration.

### Configuring the AuthKit environment

Once credentials are found, the `WORKOS_API_KEY` is used to configure the
environment based on the `configure` section of the relevant `authKit` object.
This sets things like an environment's
[redirect URIs](https://workos.com/docs/sso/redirect-uris),
[allowed CORS origins](https://workos.com/docs/authkit/client-only).

### Setting local environment variables

For dev deployments only, environment variables are written to `.env.local`
based on the `localEnvVars` section of the relevant `authKit` config.

## Project-level vs deployment level AuthKit environments

In hosting providers with remote build pipelines like Vercel, it's difficult to
set environment variables like `WORKOS_API_KEY` at build time in a way that's
available to server-side code like Next.js middleware. This makes it necessary
set the `WORKOS_*` environment variables in advance for preview and production
deployments built on these platforms.

After creating the WorkOS AuthKit environments for production and preview
deployments in the dashboard, copy relevant environment variables like
`WORKOS_CLIENT_ID`, `WORKOS_API_KEY`, `WORKOS_REDIRECT_URI`, and
`WORKOS_COOKIE_PASSWORD` to the preview and production environment variables in
your hosting provider.

Deployment-specific AuthKit environments can be created for any deployment are
difficult set up automatically so shared project-level environments are
generally a better fit.

In the `authKit` section of `convex.json`, `localEnvVars`
`automate setting up dev environments by automatically setting the right environment variables in .env.local and automatically configuring the environment with a `redirectUri`.

Environments for hosting providers in build environments like Vercel (production
and preview deploys) can be configured at build time, but the environment
variables for these build environments must be set manually in the build
settings.

## Recommended Configuration

Here's a common setup for a project where production and preview deploys are
deployed to from Vercel. Check your hosting provider's docs to substitute the
right environment variables, and check the guide for using AuthKit with your
framework of choice to customize this example.

```json title="convex.json"
{
  "authKit": {
    "dev": {
      "configure": {
        "redirectUris": ["http://localhost:3000/callback"],
        "appHomepageUrl": "http://localhost:3000",
        "corsOrigins": ["http://localhost:3000"]
      },
      "localEnvVars": {
        "WORKOS_CLIENT_ID": "${authEnv.WORKOS_CLIENT_ID}",
        "WORKOS_API_KEY": "${authEnv.WORKOS_API_KEY}",
        "NEXT_PUBLIC_WORKOS_REDIRECT_URI": "http://localhost:3000/callback"
      }
    },
    "preview": {
      "configure": {
        "redirectUris": ["https://${buildEnv.VERCEL_BRANCH_URL}/callback"],
        "appHomepageUrl": "https://${buildEnv.VERCEL_PROJECT_PRODUCTION_URL}",
        "corsOrigins": ["https://${buildEnv.VERCEL_BRANCH_URL}"]
      }
    },
    "prod": {
      "environmentType": "production",
      "configure": {
        "redirectUris": [
          "https://${buildEnv.VERCEL_PROJECT_PRODUCTION_URL}/callback"
        ],
        "appHomepageUrl": "https://${buildEnv.VERCEL_PROJECT_PRODUCTION_URL}",
        "corsOrigins": ["https://${buildEnv.VERCEL_PROJECT_PRODUCTION_URL}"]
      }
    }
  }
}
```

Additionally, for local dev in **Next.js** and **TanStack Start**, Convex
automatically generates a `WORKOS_COOKIE_PASSWORD` if it's not already in
`.env.local`.
