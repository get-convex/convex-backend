---
title: "Exception Reporting"
sidebar_label: "Exception Reporting"
sidebar_position: 3
description:
  "Configure exception reporting integrations for your Convex deployment"
---

Configure exception reporting to gain visibility into errors from your Convex
function executions. Convex supports integration with
[Sentry](https://sentry.io/) and with
[Datadog Error Tracking](https://www.datadoghq.com/product/error-tracking/).

Currently, exception reporting is only available to Pro users.

## Configuring Sentry

To configure sentry, navigate to the
[Deployment Settings](/dashboard/deployments/settings.md) in the Dashboard, and
the "Integrations" tab in the sidebar.

![Integrations Page](/screenshots/integrations_page.png)

Click on the Sentry card and follow the setup directions. You will need your
[Sentry DSN](https://docs.sentry.io/product/sentry-basics/concepts/dsn-explainer/).
You may optionally specify additional tags to be added to each exception event.

![Configure sentry](/screenshots/configure_sentry.png)

## Supported Tags

Convex automatically tags exception events on their way to sentry with the
following tags. These tags cannot be overridden.

- `func`: The name of the running function in
  [string format](/functions/query-functions#query-names)
- `func_type`: One of `["query", "mutation", "action", "http_action"]`
- `func_runtime`: One of the [function runtimes](/functions/runtimes.mdx) -
  `["default", "node"]`
- `request_id`: The
  [request id](/functions/debugging.mdx#finding-relevant-logs-by-request-id) of
  the function that errored.
- `server_name`: The name of the deployment. e.g. `happy-animal-123`
- `environment`: One of `["prod", "dev", "preview"]`
- `user`: If the function is [authenticated](/auth.mdx), then the
  [tokenIdentifier](/api/interfaces/server.UserIdentity#tokenidentifier) is used
  as the user id on Sentry. The `tokenIdentifier` is a stable and globally
  unique string representing the authenticated user.

## Sentry Notes

- Sentry Exceptions may take a minute or two to propagate to Sentry.
- Convex's built-in sentry support does not yet support the advanced
  customization provided by the sentry SDK.
- Please reach out with any questions, comments, or suggestions
  [on Discord](https://convex.dev/community).

## Configuring Datadog Error Tracking

Follow the instructions in the
[Datadog application](https://app.datadoghq.com/error-tracking/settings/setup/sentry)
to configure Datadog error tracking via the Sentry SDK. Then use the
Convex-Sentry integration to proceed.
