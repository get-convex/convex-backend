---
title: "Configuring Deployment URL"
slug: "deployment-urls"
sidebar_label: "Deployment URLs"
hidden: false
sidebar_position: 5
description: "Configuring your project to run with Convex"
---

When
[connecting to your backend](/docs/client/react.mdx#connecting-to-a-backend)
it's important to correctly configure the deployment URL.

### Create a Convex project

The first time you run

```sh
npx convex dev
```

in your project directory you will create a new Convex project.

Your new project includes two deployments: _production_ and _development_. The
_development_ deployment's URL will be saved in `.env.local` or `.env` file,
depending on the frontend framework or bundler you're using.

You can find the URLs of all deployments in a project by visiting the
[deployment settings](/docs/dashboard/deployments/settings.md) on your Convex
[dashboard](https://dashboard.convex.dev).

### Configure the client

Construct a Convex React client by passing in the URL of the Convex deployment.
There should generally be a single Convex client in a frontend application.

```jsx title="src/index.js"
import { ConvexProvider, ConvexReactClient } from "convex/react";

const deploymentURL = import.meta.env.VITE_CONVEX_URL;

const convex = new ConvexReactClient(deploymentURL);
```

While this URL can be hardcoded, it's convenient to use an environment variable
to determine which deployment the client should connect to.

Use an environment variable name accessible from your client code according to
the frontend framework or bundler you're using.

### Choosing environment variable names

To avoid unintentionally exposing secret environment variables in frontend code,
many bundlers require environment variables referenced in frontend code to use a
specific prefix.

[Vite](https://vitejs.dev/guide/env-and-mode.html) requires environment
variables used in frontend code start with `VITE_`, so `VITE_CONVEX_URL` is a
good name.

[Create React App](https://create-react-app.dev/docs/adding-custom-environment-variables/)
requires environment variables used in frontend code to begin with `REACT_APP_`,
so the code above uses `REACT_APP_CONVEX_URL`.

[Next.js](https://nextjs.org/docs/basic-features/environment-variables#exposing-environment-variables-to-the-browser)
requires them to begin with `NEXT_PUBLIC_`, so `NEXT_PUBLIC_CONVEX_URL` is a
good name.

Bundlers provide different ways to access these variables too: while
[Vite uses `import.meta.env.VARIABLE_NAME`](https://vitejs.dev/guide/env-and-mode.html),
many other tools like Next.js use the Node.js-like
[`process.env.VARIABLE_NAME`](https://nextjs.org/docs/basic-features/environment-variables)

```jsx
import { ConvexProvider, ConvexReactClient } from "convex/react";

const convex = new ConvexReactClient(process.env.NEXT_PUBLIC_CONVEX_URL);
```

[`.env` files](https://www.npmjs.com/package/dotenv) are a common way to wire up
different environment variable values in development and production
environments. `npx convex dev` will save the deployment URL to the corresponding
`.env` file, while trying to infer which bundler your project uses.

```shell title=".env.local"
NEXT_PUBLIC_CONVEX_URL=https://guiltless-dog-960.convex.cloud

# examples of other environment variables that might be passed to the frontend
NEXT_PUBLIC_SENTRY_DSN=https://123abc@o123.ingest.sentry.io/1234
NEXT_PUBLIC_LAUNCHDARKLY_SDK_CLIENT_SIDE_ID=01234567890abcdef
```

Your backend functions can use
[environment variables](/docs/production/environment-variables.mdx) configured
on your dashboard. They do not source values from `.env` files.
