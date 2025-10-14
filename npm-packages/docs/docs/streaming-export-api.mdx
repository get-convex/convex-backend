---
title: "Streaming Export"
sidebar_label: "Streaming Export"
description: "Streaming data out of Convex"
---

Convex supports streaming export. Convex provides connector implementations for
[Fivetran and Airbyte](/production/integrations/streaming-import-export.md).
Those connectors use the following APIs.

Sign up for a [Professional plan](https://www.convex.dev/pricing) for streaming
export support. You can also read the
[documentation on streaming export](/production/integrations/streaming-import-export.md).

<BetaAdmonition feature="Streaming Export HTTP APIs" verb="are" />

Streaming export requests require deployment admin authorization via the HTTP
header `Authorization`. The value is `Convex <access_key>` where the access key
comes from "Deploy key" on the Convex dashboard and gives full read and write
access to your Convex data.

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
