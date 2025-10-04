---
title: "Automatic AuthKit Configuration"
sidebar_label: "Automatic Config"
sidebar_position: 20
description: "WorkOS AuthKit authentication with Convex"
---

AuthKit configuration can be automated for cloud dev deployments: each Convex
deployment gets its own WorkOS environment configured and has local environment
variables added to `.env.local` and Convex deployment environment variables set
for it.

This integration is in active development and will change as it continues to
improve. Today the integration works with the two AuthKit templates offered when
running `npm create convex@latest`.

## Creating WorkOS environments on-demand

Automatically provisioning a WorkOS environment for a Convex deployment is
enabled by creating a new WorkOS account and team to associate with a Convex
team. Once this account has been created, any member of the Convex team can
create a WorkOS environment for their development deployments on each of the
team's projects.

This happens automatically whenever the `WORKOS_CLIENT_ID` environment variable
is read in the `convex/auth.config.ts` file but not set on the deployment during
a `convex dev`.

The CLI then makes AuthKit-related configuration changes that replace the
[manual configuration steps](/docs/auth/authkit/index.mdx#configuring-an-existing-workos-account)
required to configure AuthKit for a development Convex deployment.

Currently this configures the following with the assumed local development
domain:

- redirect endpoint URI
- CORS origin

The following local environment variables may be set in `.env.local`:

- `VITE_WORKOS_CLIENT_ID` (Vite only)
- `WORKOS_CLIENT_ID` (Next.js only)
- `*_WORKOS_REDIRECT_URI` (e.g. `VITE_WORKOS_REDIRECT_URI`)
- `WORKOS_API_KEY` (Next.js only)
- `WORKOS_COOKIE_PASSWORD` (Next.js only)

### Limitations

WorkOS environments can currently only be created for cloud development
deployments. Preview and production deployments must be manually configured.

To manually configure the production deployment, visit the WorkOS page for the
production environment for this project and
[follow these steps](/docs/auth/authkit/index.mdx#configuring-an-existing-workos-account).
Only one production deployment exists by default per WorkOS team so additional
project may need to use separate WorkOS teams.
