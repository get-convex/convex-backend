---
title: "Environment Variables"
sidebar_label: "Environment Variables"
sidebar_position: 2
description: "Store and access environment variables in Convex"
---

Environment variables are key-value pairs that are useful for storing values you
wouldn't want to put in code or in a table, such as an API key. You can set
environment variables in Convex through the dashboard, and you can access them
in [functions](/functions.mdx) using `process.env`.

## Setting environment variables

Under [Deployment Settings](/dashboard/deployments/settings.md) in the
Dashboard, you can see a list of environment variables in the current
deployment.
![Environment Variables Table](/screenshots/environment_variables_table.png)

You can add up to 100 environment variables. Environment variable names cannot
be more than 40 characters long, and they must start with a letter and only
contain letters numbers, and underscores. Environment variable values cannot be
larger than 8KB.

You can modify environment variables using the pencil icon button:

![Edit Environment Variable](/screenshots/edit_environment_variable.png)

Environment variables can also be viewed and modified with the
[command line](/cli.md#read-and-write-environment-variables).

```sh
npx convex env list
npx convex env set API_KEY secret-api-key
```

### Using environment variables in dev and prod deployments

Since environment variables are set per-deployment, you can use different values
for the same key in dev and prod deployments. This can be useful for when you
have different external accounts you'd like to use depending on the environment.
For example, you might have a dev and prod SendGrid account for sending emails,
and your function expects an environment variable called `SENDGRID_API_KEY` that
should work in both environments.

If you expect an environment variable to be always present in a function, you
must add it to **all** your deployments. In this example, you would add an
environment variable with the name `SENDGRID_API_KEY` to your dev and prod
deployments, with a different value for dev and prod.

## Accessing environment variables

You can access environment variables in Convex functions using
`process.env.KEY`. If the variable is set it is a `string`, otherwise it is
`undefined`. Here is an example of accessing an environment variable with the
key `GIPHY_KEY`:

```javascript
function giphyUrl(query) {
  return (
    "https://api.giphy.com/v1/gifs/translate?api_key=" +
    process.env.GIPHY_KEY +
    "&s=" +
    encodeURIComponent(query)
  );
}
```

Note that you should not condition your Convex function exports on environment
variables. The set of Convex functions that can be called is determined during
deployment and is not reevaluated when you change an environment variable. The
following code will throw an error at runtime, if the DEBUG environment variable
changes between deployment and calling the function.

```javascript
// THIS WILL NOT WORK!
export const myFunc = process.env.DEBUG ? mutation(...) : internalMutation(...);
```

Similarly, environment variables used in cron definitions will only be
reevaluated on deployment.

## System environment variables

The following environment variables are always available in Convex functions:

- `CONVEX_CLOUD_URL` - Your deployment URL (eg.
  `https://dusty-nightingale-847.convex.cloud`) for use with Convex clients.
- `CONVEX_SITE_URL` - Your deployment site URL (eg.
  `https://dusty-nightingale-847.convex.site`) for use with
  [HTTP Actions](/functions/http-actions.mdx)

## Project environment variable defaults

You can set up default environment variable values for a project for development
and preview deployments in Project Settings.

![Project Default Environment Variables](/screenshots/project_default_environment_variables.png)

These default values will be used when creating a new development or preview
deployment, and will have no effect on existing deployments (they are not kept
in sync).

The Deployment Settings will indicate when a deployment has environment
variables that do not match the project defaults.
![Environment Variable Default Mismatch](/screenshots/environment_variable_default_diff.png)
