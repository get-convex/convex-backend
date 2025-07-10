---
title: "Local Deployments for Development"
slug: "local-deployments"
sidebar_position: 10
sidebar_label: "Local Deployments"
---

Instead of syncing code to a Convex dev deployment hosted in the cloud, you can
develop against a deployment running on your own computer. You can even use the
Convex dashboard with local deployments!

## Background on deployments in Convex

Each Convex deployment contains its own data, functions, scheduled functions,
etc. A project has one production deployment, up to one cloud deployment for
development per team member, and potentially many transient
[preview deployments](/production/hosting/preview-deployments.mdx).

You can also develop with Convex using a deployment running on your own machine.
Since the deployment is running locally, code sync is faster and means resources
like functions calls and database bandwidth don't count against
[the quotas for your Convex plan](https://www.convex.dev/pricing).

You can use local deployments with an existing Convex project, and view your
deployment in the Convex dashboard under your project. You can also use local
deployments without a Convex account and debug and inspect them with a locally
running version of the Convex dashboard.

## Using local deployments

<BetaAdmonition feature="Local deployments" verb="are" />

While using local deployments, the local Convex backend runs as a subprocess of
the `npx convex dev` command and exits when that command is stopped. This means
a `convex dev` command must be running in order to run other commands like
`npx convex run` against this local deployment or for your frontend to connect
to this deployment.

State for local backends is stored the `~/.convex/` directory.

### Anonymous development

You can use local deployments to develop with Convex without having to create an
account. Whenever you want to create an account to deploy your app to production
or to use more Convex features, you can use `npx convex login` to link your
local deployments with your account.

### Local deployments for an existing project

To use a local deployment for an existing project, run:

```sh
npx convex dev --local --once
```

You'll also always be given the option for a local deployment if you run
`npx convex dev --configure`. Other flows may assume you want a cloud deployment
in some situations, for example when connecting to a project for which you
already have a cloud development deployment.

## Local deployments vs. production

Local deployments are not recommended for production use: they're development
deployments, i.e. logs for function results and full stack traces for error
responses are sent to connected clients.

For running a production application, you can use a production deployment hosted
on the Convex cloud. Learn more about deploying to production
[here](/production.mdx).

Alternatively, you can self-host a production deployment using the
[open source convex-backend repo](https://github.com/get-convex/convex-backend).

### Disabling

To stop using local developments for a project, run the following:

```sh
npx convex disable-local-deployments
```

Remember your cloud dev deployment and each local dev deployment are completely
separate, so contain different data. When switching between deployments you may
wish to [export and re-import](/database/import-export/import-export.mdx) the
data to keep using it.

## Limitations

- **No Public URL** - Cloud deployments have public URL to receive incoming HTTP
  requests from services like Twilio, but local deployments listen for HTTP
  requests on your own computer. Similarly, you can't power websites with Convex
  WebSocket connections unless your users browsers know how to reach your
  computer. Set up a proxy like ngrok or use a cloud deployment for these uses
  cases.

- **Node actions require specific Node.js versions** - Running Node.js actions
  (actions defined in files with `"use node;"`) requires having Node.js 18
  installed, since this is the version of Node.js used in production when
  Node.js actions run in AWS Lambda functions. To resolve this you can install
  and set up [nvm](https://github.com/nvm-sh/nvm) and then install Node.js
  version 18. You don't need to use Node.js 18 for the rest of your project.

- **Node.js actions run directly on your computer** - Like a normal Node.js
  server, code running in Node.js actions has unrestricted filesystem access.
  Queries, mutations, and Convex runtime actions still run in isolated
  environments.

- Logs get cleared out every time a `npx convex dev` command is restarted.

- <a id="safari"></a> **Using the dashboard with Safari**: Safari [blocks
  requests to localhost](https://bugs.webkit.org/show_bug.cgi?id=171934), which
  prevents the dashboard from working with local deployments. We recommend using
  another browser if you’re using local deployments.

- <a id="brave"></a> **Using the dashboard with Brave**: Brave [blocks requests
  to localhost by
  default](https://brave.com/privacy-updates/27-localhost-permission/), which
  prevents the dashboard from working with local deployments. You can use the
  following workaround:
  - Go to `brave://flags/`
  - Enable the `#brave-localhost-access-permission` flag
  - Go back to the Convex dashboard
  - Click on **View Site Information**
    (<img src="/screenshots/brave-site-information.png" alt="View Site Information icon" width={24} style={{ verticalAlign: "middle" }} />)
    in the URL bar, then on **Site settings**
  - Change the setting for **Localhost access** to **Allow**
