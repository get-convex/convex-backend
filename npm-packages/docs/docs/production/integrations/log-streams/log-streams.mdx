---
title: "Log Streams"
sidebar_label: "Log Streams"
sidebar_position: 2
description: "Configure logging integrations for your Convex deployment"
---

Log streams enable streaming of events such as function executions and
`console.log`s from your Convex deployment to supported destinations, such as
Axiom, Datadog, or a custom webhook.

The most recent logs produced by your Convex deployment can be viewed in the
Dashboard [Logs page](/docs/dashboard/deployments/logs.md), the
[Convex CLI](/docs/cli.md), or in the browser console, providing a quick and
easy way to view recent logs.

Log streaming to a third-party destination like Axiom or Datadog enables storing
historical logs, more powerful querying and data visualization, and integrations
with other tools (e.g. PagerDuty, Slack).

<ProFeatureUpsell feature="Log streams" verb="require" />

## Configuring log streams

We currently support the following log streams, with plans to support many more:

- [Axiom](https://www.axiom.co)
- [Datadog](https://www.datadoghq.com/)
- Webhook to a custom URL

See the instructions for
[configuring an integration](/docs/production/integrations/integrations.mdx#configuring-an-integration).
The specific information needed for each log stream is covered below.

### Axiom

Configuring an Axiom log stream requires specifying:

- The name of your
  [Axiom dataset](https://axiom.co/docs/reference/settings#dataset)
- An Axiom [API key](https://axiom.co/docs/reference/settings#api-token)
- An optional list of attributes and their values to be included in all log
  events send to Axiom. These will be sent via the `attributes` field in the
  [Ingest API](https://axiom.co/docs/send-data/ingest#ingest-api).

### Datadog

Configuring a Datadog log stream requires specifying:

- The [site location](https://docs.datadoghq.com/getting_started/site/) of your
  Datadog deployment
- A Datadog
  [API key](https://docs.datadoghq.com/account_management/api-app-keys/#add-an-api-key-or-client-token)
- A comma-separated list of tags that will be passed using the
  [`ddtags` field](https://docs.datadoghq.com/getting_started/tagging/) in all
  payloads sent to Datadog. This can be used to include any other metadata that
  can be useful for querying or categorizing your Convex logs ingested by your
  Datadog deployment.

### Webhook

A webhook log stream is the simplest and most generic stream, allowing piping
logs via POST requests to any URL you configure. The only parameter required to
set up this stream is the desired webhook URL.

A request to this webhook contains as its body a JSON array of events in the
schema defined below.

## Log event schema

<Admonition type="info">
  Log streams configured before May 23, 2024 will use the legacy format
  documented on [this
  page](/docs/production/integrations/log-streams/legacy-event-schema.mdx). We
  recommend updating your log stream to use the new format.
</Admonition>

Log events have a well-defined JSON schema that allow building complex,
type-safe pipelines ingesting log events.

All events will have the following two fields:

- `topic`: string, categorizes a log event, one of
  `["verification", "console", "function_execution", "audit_log"]`
- `timestamp`: number, Unix epoch timestamp in milliseconds as an integer

### `verification` events

This is an event sent to confirm the log stream is working. Schema:

- `topic`: `"verification"`
- `timestamp`: Unix epoch timestamp in milliseconds
- `message`: string

### `console` events

Convex function logs via the [`console` API](/docs/functions/debugging.mdx).

Schema:

- `topic`: `"console"`
- `timestamp`: Unix epoch timestamp in milliseconds
- `function`: object, see
  [function fields](/docs/production/integrations/log-streams/log-streams.mdx#function-fields)
- `log_level`: string, one of `["DEBUG", "INFO", "LOG", "WARN", "ERROR"]`
- `message`: string, the
  [`object-inspect`](https://www.npmjs.com/package/object-inspect)
  representation of the `console.log` payload
- `is_truncated`: boolean, whether this message was truncated to fit within our
  logging limits
- `system_code`: optional string, present for automatically added warnings when
  functions are approaching
  [limits](/docs/production/state/limits.mdx#functions)

Example event for `console.log("Sent message!")` from a mutation:

```json
{
    "topic": "console"
    "timestamp": 1715879172882,
    "function": {
      "path": "messages:send",
      "request_id": "d064ef901f7ec0b7",
      "type": "mutation"
    },
    "log_level": "LOG",
    "message": "'Sent message!'"
}
```

### `function_execution` events

These events occur whenever a function is run.

Schema:

- `topic`: `"function_execution"`
- `timestamp`: Unix epoch timestamp in milliseconds
- `function`: object, see
  [function fields](/docs/production/integrations/log-streams/log-streams.mdx#function-fields)
- `execution_time_ms`: number, the time in milliseconds this function took to
  run
- `status`: string, one of `["success", "failure"]`
- `error_message`: string, present for functions with status `failure`,
  containing the error and any stack trace.
- `mutation_queue_length`: optional number (for mutations only), the length of
  the per-session mutation queue at the time the mutation was executed. This is
  useful for monitoring and debugging mutation queue backlogs in individual
  sessions.
- `usage`:
  - `database_read_bytes`: number
  - `database_write_bytes`: number, this and `database_read_bytes` make up the
    database bandwidth used by the function
  - `file_storage_read_bytes`: number
  - `file_storage_write_bytes`: number, this and `file_storage_read_bytes` make
    up the file bandwidth used by the function
  - `vector_storage_read_bytes`: number
  - `vector_storage_write_bytes`: number, this and `vector_storage_read_bytes`
    make up the vector bandwidth used by the function
  - `action_memory_used_mb`: number, for actions, the memory used in MiB. This
    combined with `execution_time_ms` makes up the action compute.

Example event for a query:

```json
{
  "data": {
    "execution_time_ms": 294,
    "function": {
      "cached": false,
      "path": "message:list",
      "request_id": "892104e63bd39d9a",
      "type": "query"
    },
    "status": "success",
    "timestamp": 1715973841548,
    "topic": "function_execution",
    "usage": {
      "database_read_bytes": 1077,
      "database_write_bytes": 0,
      "file_storage_read_bytes": 0,
      "file_storage_write_bytes": 0,
      "vector_storage_read_bytes": 0,
      "vector_storage_write_bytes": 0
    }
  }
}
```

### Function fields

The following fields are added under `function` for all `console` and
`function_execution` events:

- `type`: string, one of `["query", "mutation", "action", "http_action"]`
- `path`: string, e.g. `"myDir/myFile:myFunction"`, or `"POST /my_endpoint"`
- `cached`: optional boolean, for queries this denotes whether this event came
  from a cached function execution
- `request_id`: string, the
  [request ID](/docs/functions/debugging.mdx#finding-relevant-logs-by-request-id)
  of the function.

### `audit_log` events

These events represent changes to your deployment, which also show up in the
[History tab](https://dashboard.convex.dev/deployment/history) in the dashboard.

Schema:

- `topic`: `audit_log`
- `timestamp`: Unix epoch timestamp in milliseconds
- `audit_log_action`: string, e.g. `"create_environment_variable"`,
  `"push_config"`, `"change_deployment_state"`
- `audit_log_metadata`: string, stringified JSON holding metadata about the
  event. The exact format of this event may change.

Example `push_config` audit log:

```json
{
  "topic": "audit_log",
  "timestamp": 1714421999886,
  "audit_log_action": "push_config",
  "audit_log_metadata": "{\"auth\":{\"added\":[],\"removed\":[]},\"crons\":{\"added\":[],\"deleted\":[],\"updated\":[]},..."
}
```

## Guarantees

Log events provide a best-effort delivery guarantee. Log streams are buffered
in-memory and sent out in batches to your deployment's configured streams. This
means that logs can be dropped if ingestion throughput is too high. Similarly,
due to network retries, it is possible for a log event to be duplicated in a log
stream.

That's it! Your logs are now configured to stream out. If there is a log
streaming destination that you would like to see supported,
[please let us know](/docs/production/contact.md)!

<StackPosts query="axiom" />
