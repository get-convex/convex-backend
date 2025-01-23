# (Legacy) Event schema

<Admonition type="info">
  Log streams configured before May 23, 2024 will use the legacy format
  documented here. We recommend updating your log stream to use the new format.
</Admonition>

## Updating to the new format

You can update existing log streams to the new format in the dashboard under
your [deployment's Settings](https://dashboard.convex.dev/deployment/settings) >
Integrations.

You can either create an entirely new dataset to hold events using the new
format, or can reuse your existing dataset to hold historical events in the
legacy format as well as events in the new format going forward.

We recommend reading the documentation on both the legacy format and the
[current format](/docs/production/integrations/log-streams/log-streams.mdx#log-event-schema)
for the full set of differences, but here are a few key differences:

- Many fields have been renamed to drop leading underscores and use `snake_case`
- Fields have been added, e.g.
  - `function.request_id`
  - `usage.vector_storage_read_bytes`
  - `log_level`
- Fields have been renamed or nested for clarity, e.g.
  - `reason` -> `error_message`
  - `_functionPath` -> `function.path`

## (Legacy) Event schema

Log events have a well-defined JSON schema that allow building complex,
type-safe pipelines ingesting log events.

## System fields

System fields are reserved fields which are included on log events and prefixed
by an underscore.

All log events include the following system fields:

- `_topic`: string that categorizes a log event by its internal source
- `_timestamp`: Unix epoch timestamp in milliseconds. This is as an integer.

## Log sources

This section outlines the source and data model of all log events.

### `console` logs

Convex function logs via the `console` API.

Schema:

- `_topic = "_console"`
- `_timestamp` = Unix epoch timestamp in milliseconds
- `_functionType = "query" | "mutation" | "action" | "httpAction"`
- `_functionPath` =
  - If this is an HTTP action, this is a string of the HTTP method and URL
    pathname i.e. `POST /my_endpoint`
  - Otherwise, this is a path to function within `convex/` directory including
    an optional module export identifier i.e. `myDir/myFile:myFunction`.
- `_functionCached = true | false`. This field is only set if
  `_functionType = "query"` and says if this log event came from a cached
  function execution.
- `message` = payload string of arguments to `console` API

Example query log event:

```json
{
  "_topic": "_console",
  "_timestamp": 1695066350531,
  "_functionType": "query",
  "_functionPath": "myDir/myFile",
  "_functionCached": true,
  "message": "[LOG] 'My log message'"
}
```

### Function execution record logs

Function executions which log a record of their execution and their result.

Schema:

- `_topic = "_execution_record"`
- `_timestamp` = Unix epoch timestamp in milliseconds
- `_functionType = "query" | "mutation" | "action" | "httpAction"`
- `_functionPath` = path to function within `convex/` directory including module
  export identifier
- `_functionCached = true | false`. This field is only set if
  `_functionType = "query"` and says if this log event came from a cached
  function execution.
- `status = "success" | "failure"`
- `reason` = error message from function. Only set if `status = "failure"`
- `executionTimeMs` = length of execution of this function in milliseconds
- `databaseReadBytes` = the database read bandwidth used by this function in
  bytes
- `databaseWriteBytes` = the database write bandwidth used by this function in
  bytes
- `storageReadBytes` = the file storage read bandwidth this function used in
  bytes
- `storageWriteBytes` = the file storage write bandwidth this function used in
  bytes

Example execution record log from an HTTP action:

```json
{
  "_topic": "_execution_record",
  "_timestamp": 1695066350531,
  "_functionType": "httpAction",
  "_functionPath": "POST /sendImage",
  "status": "failure",
  "reason": "Unexpected Error: Some error message\n\n  at ....",
  "executionTimeMs": 73
}
```

### Audit trail logs

Audit logs of deployment events.

Schema:

- `_topic = "_audit_log"`
- `_timestamp` = Unix epoch timestamp in milliseconds
- `action = "create_environment_variable" | "update_environment_variable" | "delete_environment_variable" | "replace_environment_variable" | "push_config" | "build_indexes" | "change_deployment_state"`
- `actionMetadata` = object whose fields depends on the value of the `action`
  field.

Example `push_config` audit log:

```json
{
  "_topic": "_audit_log",
  "_timestamp": 1695066350531,
  "action": "push_config",
  "actionMetadata": {
    "modules": {
      "added": ["ffmpeg.js", "fetch.js", "test.js"],
      "removed": ["removed.js"]
    }
  }
}
```

### Verification logs

Internal logging events used to verify access to a log stream.

Schema

- `_topic = "_verification"`
- `_timestamp` = Unix epoch timestamp in milliseconds.
- `message = Convex connection test`
