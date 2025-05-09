---
title: "Convex HTTP API"
sidebar_label: "HTTP API"
description: "Connecting to Convex directly with HTTP"
---

import Tabs from '@theme/Tabs'; import TabItem from '@theme/TabItem';

# HTTP APIs

HTTP APIs include:

- [Functions API](#functions-api)
- [Streaming export API](#streaming-export-api)
- [Streaming import API](#streaming-import-api)

## Convex value format

Each of the HTTP APIs take a `format` query param that describes how documents
are formatted. Currently the only supported value is `json`. See our
[types page](/database/types#convex-values) for details. Note that for
simplicity, the `json` format does not support all Convex data types as input,
and uses overlapping representation for several data types in output. We plan to
add a new format with support for all Convex data types in the future.

## API authentication

The Functions API can be optionally authenticated as a user via a bearer token
in a `Authorization` header. The value is `Bearer <access_key>` where the key is
a token from your auth provider. See the
[under the hood](/auth/clerk#under-the-hood) portion of the Clerk docs for
details on how this works with Clerk.

Streaming export and streaming import requests require deployment admin
authorization via the HTTP header `Authorization`. The value is
`Convex <access_key>` where the access key comes from "Deploy key" on the Convex
dashboard and gives full read and write access to your Convex data.

## Functions API

### POST `/api/query`, `/api/mutation`, `/api/action`

These HTTP endpoints allow you to call Convex functions and get the result as a
value.

You can find your backend deployment URL on the dashboard
[Settings](/dashboard/deployments/settings.md) page, then the API URL will be
`<CONVEX_URL>/api/query` etc., for example:

<Tabs>
<TabItem value="shell" label="Shell">

```
curl https://acoustic-panther-728.convex.cloud/api/query \
   -d '{"path": "messages:list", "args": {}, "format": "json"}' \
   -X POST -H "Content-Type: application/json"
```

</TabItem>
<TabItem value="js" label="NodeJS">

```js
const url = "https://acoustic-panther-728.convex.cloud/api/query";
const request = { path: "messages:list", args: {}, format: "json" };

const response = fetch(url, {
  method: "POST",
  headers: {
    "Content-Type": "application/json",
  },
  body: JSON.stringify(request),
});
```

</TabItem>
<TabItem value="py" label="Python">

```py
import requests

url = "https://acoustic-panther-728.convex.cloud/api/query"
headers = {"accept": "application/json"}
body = {"path": "messages:list", "args": {}, "format": "json"}

response = requests.post(url, headers=headers, json=body)
```

</TabItem>
</Tabs>

**JSON Body parameters**

| Name   | Type   | Required | Description                                                                                                  |
| ------ | ------ | -------- | ------------------------------------------------------------------------------------------------------------ |
| path   | string | y        | Path to the Convex function formatted as a string as defined [here](/functions/query-functions#query-names). |
| args   | object | y        | Named argument object to pass to the Convex function.                                                        |
| format | string | n        | Output format for values. Valid values: [`json`]                                                             |

**Result JSON on success**

| Field Name | Type         | Description                                            |
| ---------- | ------------ | ------------------------------------------------------ |
| status     | string       | "success"                                              |
| value      | object       | Result of the Convex function in the requested format. |
| logLines   | list[string] | Log lines printed out during the function execution.   |

**Result JSON on error**

| Field Name   | Type         | Description                                                                                              |
| ------------ | ------------ | -------------------------------------------------------------------------------------------------------- |
| status       | string       | "error"                                                                                                  |
| errorMessage | string       | The error message.                                                                                       |
| errorData    | object       | Error data within an [application error](/functions/error-handling/application-errors) if it was thrown. |
| logLines     | list[string] | Log lines printed out during the function execution.                                                     |

### POST `/api/run/{functionIdentifier}`

This HTTP endpoint allows you to call arbitrary Convex function types with the
path in the request URL and get the result as a value. The function identifier
is formatted as a string as defined
[here](/functions/query-functions#query-names) with a `/` replacing the `:`.

You can find your backend deployment URL on the dashboard
[Settings](/dashboard/deployments/settings.md) page, then the API URL will be
`<CONVEX_URL>/api/run/{functionIdentifier}` etc., for example:

<Tabs>
<TabItem value="shell" label="Shell">

```
curl https://acoustic-panther-728.convex.cloud/api/run/messages/list \
   -d '{"args": {}, "format": "json"}' \
   -X POST -H "Content-Type: application/json"
```

</TabItem>
<TabItem value="js" label="NodeJS">

```js
const url = "https://acoustic-panther-728.convex.cloud/api/run/messages/list";
const request = { args: {}, format: "json" };

const response = fetch(url, {
  method: "POST",
  headers: {
    "Content-Type": "application/json",
  },
  body: JSON.stringify(request),
});
```

</TabItem>
<TabItem value="py" label="Python">

```py
import requests

url = "https://acoustic-panther-728.convex.cloud/api/run/messages/list"
headers = {"accept": "application/json"}
body = {"args": {}, "format": "json"}

response = requests.get(url, headers=headers, body=json)
```

</TabItem>
</Tabs>

**JSON Body parameters**

| Name   | Type   | Required | Description                                                          |
| ------ | ------ | -------- | -------------------------------------------------------------------- |
| args   | object | y        | Named argument object to pass to the Convex function.                |
| format | string | n        | Output format for values. Defaults to `json`. Valid values: [`json`] |

**Result JSON on success**

| Field Name | Type         | Description                                            |
| ---------- | ------------ | ------------------------------------------------------ |
| status     | string       | "success"                                              |
| value      | object       | Result of the Convex function in the requested format. |
| logLines   | list[string] | Log lines printed out during the function execution.   |

**Result JSON on error**

| Field Name   | Type         | Description                                                                                              |
| ------------ | ------------ | -------------------------------------------------------------------------------------------------------- |
| status       | string       | "error"                                                                                                  |
| errorMessage | string       | The error message.                                                                                       |
| errorData    | object       | Error data within an [application error](/functions/error-handling/application-errors) if it was thrown. |
| logLines     | list[string] | Log lines printed out during the function execution.                                                     |

## Streaming export API

Convex supports streaming export. Convex provides connector implementations for
[Fivetran and Airbyte](/production/integrations/streaming-import-export.md).
Those connectors use the following APIs.

Sign up for a [Professional plan](https://www.convex.dev/pricing) for streaming
export support. You can also read the
[documentation on streaming export](/production/integrations/streaming-import-export.md).

<BetaAdmonition feature="Streaming Export HTTP APIs" verb="are" />

### GET `/api/json_schemas`

The JSON Schemas endpoint lists tables, and for each table describes how
documents will be encoded, given as [JSON Schema](https://json-schema.org/).
This endpoint returns `$description` tags throughout the schema to describe
unintuitive encodings and give extra information like the table referenced by
`Id` fields.

**Query parameters**

| Name        | Type    | Required | Description                                                                                                     |
| ----------- | ------- | -------- | --------------------------------------------------------------------------------------------------------------- |
| deltaSchema | boolean | n        | If set, include metadata fields returned by document_deltas and list_snapshot (`_ts`, `_deleted`, and `_table`) |
| format      | string  | n        | Output format for values. Valid values: [`json`]                                                                |

### GET `/api/list_snapshot`

The `list_snapshot` endpoint walks a consistent snapshot of documents. It may
take one or more calls to `list_snapshot` to walk a full snapshot.

**Query parameters**

| Name      | Type   | Required | Description                                                                                                                                  |
| --------- | ------ | -------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| snapshot  | int    | n        | Database timestamp at which to continue the snapshot. The timestamp must not be older than 30 days. If omitted, select the latest timestamp. |
| cursor    | string | n        | An opaque cursor representing the progress in paginating through the snapshot. If omitted, start from the first page of the snapshot.        |
| tableName | string | n        | If provided, filters the snapshot to a table. If omitted, provide snapshot across all tables.                                                |
| format    | string | n        | Output format for values. Valid values: [`json`]                                                                                             |

**Result JSON**

| Field Name | Type              | Description                                                                                             |
| ---------- | ----------------- | ------------------------------------------------------------------------------------------------------- |
| values     | List[ConvexValue] | List of convex values in the requested format. Each value includes extra fields `_ts` and `_table`.     |
| hasMore    | boolean           | True if there are more pages to the snapshot.                                                           |
| snapshot   | int               | A value that represents the database timestamp at which the snapshot was taken.                         |
| cursor     | string            | An opaque cursor representing the end of the progress on the given page. Pass this to subsequent calls. |

Expected API usage (pseudocode):

```python
def list_full_snapshot()
    snapshot_values = []
    snapshot = None
    cursor = None
    while True:
        result = api.list_snapshot(cursor, snapshot)
        snapshot_values.extend(result.values)
        (cursor, snapshot) = (result.cursor, result.snapshot)
        if !result.hasMore:
            break
    return (snapshot_values, result.snapshot)
```

### GET `/api/document_deltas`

The `document_deltas` endpoint walks the change log of documents to find new,
updated, and deleted documents in the order of their mutations. This order is
given by a `_ts` field on the returned documents. Deletions are represented as
JSON objects with fields `_id`, `_ts`, and `_deleted: true`.

**Query parameters**

| Name      | Type   | Required | Description                                                                                                                              |
| --------- | ------ | -------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| cursor    | int    | y        | Database timestamp after which to continue streaming document deltas. Initial value is the `snapshot` field returned from list_snapshot. |
| tableName | string | n        | If provided, filters the document deltas to a table. If omitted, provide deltas across all tables.                                       |
| format    | string | n        | Output format for values. Valid values: [`json`]                                                                                         |

**Result JSON**

| Field Name | Type              | Description                                                                                                                                    |
| ---------- | ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| values     | List[ConvexValue] | List of convex values in the requested format. Each value includes extra fields for `_ts`, and `_table`. Deletions include a field `_deleted`. |
| hasMore    | boolean           | True if there are more pages to the snapshot.                                                                                                  |
| cursor     | int               | A value that represents the database timestamp at the end of the page. Pass to subsequent calls to document_deltas.                            |

Expected API usage (pseudocode):

```python
def delta_sync(delta_cursor):
    delta_values = []
    while True:
        result = api.document_deltas(cursor)
        delta_values.extend(result.values)
        cursor = result.cursor
        if !hasMore:
            break
    return (delta_values, delta_cursor)

(snapshot_values, delta_cursor) = list_full_snapshot()
(delta_values, delta_cursor) = delta_sync(delta_cursor)
# Save delta_cursor for the next sync
```

## Streaming import API

Convex supports streaming import. Convex provides a connector implementation for
[Airbyte](/production/integrations/streaming-import-export.md). Those connectors
use the following APIs.

Streaming import support is automatically enabled for all Convex projects.

### Headers

Streaming import endpoints accept a `Convex-Client: streaming-import-<version>`
header, where the version follows [Semver](https://semver.org/) guidelines. If
this header is not specified, Convex will default to the latest version. We
recommend using the header to ensure the consumer of this API does not break as
the API changes.

### GET `/api/streaming_import/primary_key_indexes_ready`

The `primary_key_indexes_ready` endpoint takes a list of table names and returns
true if the primary key indexes (created by `add_primary_key_indexes`) on all
those tables are ready. If the tables are newly created, the indexes should be
ready immediately; however if there are existing documents in the tables, it may
take some time to backfill the primary key indexes. The response looks like:

```json
{
  "indexesReady": true
}
```

### PUT `/api/streaming_import/add_primary_key_indexes`

The `add_primary_key_indexes` endpoint takes a JSON body containing the primary
keys for tables and creates indexes on the primary keys to be backfilled. Note
that they are not immediately ready to query - the `primary_key_indexes_ready`
endpoint needs to be polled until it returns True before calling
`import_airbyte_records` with records that require primary key indexes. Also
note that Convex queries will not have access to these added indexes. These are
solely for use in `import_airbyte_records`. The body takes the form of a map of
index names to list of field paths to index. Each field path is represented by a
list of fields that can represent nested field paths.

```json
{
  "indexes": {
    "<table_name>": [["<field1>"], ["<field2>", "<nested_field>"]]
  }
}
```

Expected API Usage:

1. Add indexes for primary keys by making a request to
   `add_primary_key_indexes`.
2. Poll `primary_key_indexes_ready` until the response is true.
3. Query using the added indexes.

### PUT `api/streaming_import/clear_tables`

The `clear_tables` endpoint deletes all documents from the specified tables.
Note that this may require multiple transactions. If there is an intermediate
error only some documents may be deleted. The JSON body to use this API request
contains a list of table names:

```json
{
  "tableNames": ["<table_1>", "<table_2>"]
}
```

### POST `api/streaming_import/replace_tables`

This endpoint is no longer supported. Use `api/streaming_import/clear_tables`
instead.

The `replace_tables` endpoint renames tables with temporary names to their final
names, deleting any existing tables with the final names.

The JSON body to use this API request contains a list of table names:

```json
{
  "tableNames": { "<table_1_temp>": "<table_1>", "<table_2_temp>": "<table_2>" }
}
```

### POST `api/streaming_import/import_airbyte_records`

The `import_airbyte_records` endpoint enables streaming ingress into a Convex
deployment and is designed to be called from an Airbyte destination connector.

It takes a map of streams and a list of messages in the JSON body. Each stream
has a name and JSON schema that will correspond to a Convex table. Streams where
records should be deduplicated include a primary key as well, which is
represented as a list of lists of strings that are field paths. Records for
streams without a primary key are appended to tables; records for streams with a
primary key replace an existing record where the primary key value matches or
are appended if there is no match. If you are using primary keys, you must call
the `add_primary_key_indexes` endpoint first and wait for them to backfill by
polling `primary_key_indexes_ready`.

Each message contains a stream name and a JSON document that will be inserted
(or replaced, in the case of deduplicated sync) into the table with the
corresponding stream name. Table names are same as the stream names. Airbyte
records become Convex documents.

```json
{
   "tables": {
      "<stream_name>": {
         "primaryKey": [["<field1>"], ["<field2>", "<nested_field>"]],
         "jsonSchema": // see https://json-schema.org/ for examples
      }
   },
   "messages": [{
      "tableName": "<table_name>",
      "data": {} // JSON object conforming to the `json_schema` for that stream
   }]
}
```

Similar to `clear_tables`, it is possible to execute a partial import using
`import_airbyte_records` if there is a failure after a transaction has
committed.

Expected API Usage:

1. [Optional] Add any indexes if using primary keys and
   [deduplicated sync](https://docs.airbyte.com/understanding-airbyte/connections/incremental-deduped-history/)
   (see `add_primary_key_indexes` above).
2. [Optional] Delete all documents in specified tables using `clear_tables` if
   using
   [overwrite sync](https://docs.airbyte.com/understanding-airbyte/connections/full-refresh-overwrite).
3. Make a request to `import_airbyte_records` with new records to sync and
   stream information.
