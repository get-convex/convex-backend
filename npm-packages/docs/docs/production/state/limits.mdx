---
title: "Limits"
sidebar_position: 2
---

We’d love for you to have _unlimited_ joy building on Convex but engineering
practicalities dictate a few limits. This page outlines current limits in the
Convex ecosystem.

Many of these limits will become more permissive over time. Please get in touch
if any are prohibitive for your application.

Limits are applied per team unless stated otherwise.

## Team

|            | Free/Starter | Professional             |
| ---------- | ------------ | ------------------------ |
| Developers | 1-6          | 25$ per member per month |
| Projects   | 20           | 100                      |

## Database

|                   | Free/Starter                                                      | Professional                                       | Notes                                                                                     |
| ----------------- | ----------------------------------------------------------------- | -------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Storage           | 0.5 GiB included<br/>(on Starter: $0.22/month per additional GiB) | 50 GiB included<br/>$0.20/month per additional GiB | Includes database rows and indexes but not files or backups.                              |
| Bandwidth         | 1 GiB/month included<br/>(on Starter: $0.22 per additional GiB)   | 50 GiB/month included<br/>$0.20 per additional GiB | Document and index data transferred between Convex functions and the underlying database. |
| Tables            | 10,000                                                            | 10,000                                             | Per deployment.                                                                           |
| Indexes per table | 32                                                                | 32                                                 |                                                                                           |
| Fields per index  | 16                                                                | 16                                                 |
| Index name length | 64 characters                                                     | 64 characters                                      |                                                                                           |

### Restrictions

- Table and index names must be valid identifiers and cannot start with an
  underscore.

## Documents

Applied per document and to any nested `Object` unless stated otherwise.

|                     |               | Notes                                                            |
| ------------------- | ------------- | ---------------------------------------------------------------- |
| Size                | 1 MiB         |                                                                  |
| Fields              | 1024          | The number of fields/keys                                        |
| Field name length   | 64 characters | Nested `Object` keys can have length up to 1024 characters.      |
| Field nesting depth | 16            | How many times objects and arrays can be nested, e.g. `[[[[]]]]` |
| Array elements      | 8192          |                                                                  |

### Restrictions

- Field names must only contain non-control alphanumeric ASCII characters and
  underscores and must start with an alphabetic character or underscore.
- Documents cannot contain top-level fields that start with an underscore, other
  than the system-provided `_id` and `_creationTime` fields.
- Strings must be valid Unicode sequences with no unpaired surrogates.

## Functions

|                                                                              | Free/Starter                                                              | Professional                                              | Notes                                                                                                         |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Function calls                                                               | 1,000,000/month included<br/>(on Starter: $2.20 per additional 1,000,000) | 25,000,000/month included<br/>$2 per additional 1,000,000 | Explicit client calls, scheduled executions, subscription updates, and file accesses count as function calls. |
| Action execution                                                             | 20 GiB-hours included<br/>(on Starter: $0.33/GiB-hour additional)         | 250 GiB-hours included<br/>$0.30/GiB-hour additional      | Convex runtime: 64 MiB RAM.<br/>Node.js runtime: 512 MiB RAM.                                                 |
| Code size                                                                    | 32 MiB                                                                    | 32 MiB                                                    | Per deployment.                                                                                               |
| Function argument size                                                       | 16 MiB                                                                    | 16 MiB                                                    | Node actions only support arguments up to 5MiB.                                                               |
| Function return value size                                                   | 16 MiB                                                                    | 16 MiB                                                    |                                                                                                               |
| HTTP action response size                                                    | 20 MiB                                                                    | 20 MiB                                                    | There is no specific limit on request size                                                                    |
| Length of a console.log line                                                 | 4 KiB                                                                     | 4 KiB                                                     |                                                                                                               |
| [Log streaming](/production/integrations/log-streams/log-streams.mdx) limits | 4096 logs, flushed every 10 seconds                                       | 4096 logs, flushed every 10 seconds                       | How many logs can be buffered when streaming                                                                  |

## Concurrent Function Executions

Number of functions you can run at a time for each function type.

|                        | Free/Starter | Professional<sup>\*</sup> |
| ---------------------- | ------------ | ------------------------- |
| Queries                | 16           | 256                       |
| Mutations              | 16           | 256                       |
| Convex runtime actions | 64           | 256                       |
| Node actions           | 64           | 1000                      |
| HTTP actions           | 16           | 128                       |
| Scheduled jobs         | 10           | 300                       |

<sup>*</sup> Limits can be lifted for Professional plan customers on a
case-by-case basis. Send us support message through the dashboard to discuss
your needs. Usually this is only needed if your product has highly bursty
traffic.

## Execution time and scheduling

|                                              |                 | Notes                                                                                                                   |
| -------------------------------------------- | --------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Query/mutation execution time                | 1 second        | Limit applies only to user code and doesn’t include database operations.                                                |
| Action execution time                        | 10&nbsp;minutes |                                                                                                                         |
| Scheduled functions                          | 1000            | The number of other functions a single mutation can schedule.                                                           |
| Total size of scheduled functions' arguments | 16 MiB          | Applies only to mutations.                                                                                              |
| Concurrent IO operations per function        | 1000            | The number of IO operations a single function can perform, e.g., a database operation, or a fetch request in an action. |
| Outstanding scheduled functions              | 1,000,000       |                                                                                                                         |

## Transactions

These limits apply to each `query` or `mutation` function.

|                            |        | Notes                                                     |
| -------------------------- | ------ | --------------------------------------------------------- |
| Data read                  | 16 MiB | Data not returned due to a `filter` counts as scanned     |
| Data written               | 16 MiB |
| Documents scanned          | 32,000 | Documents not returned due to a `filter` count as scanned |
| Documents written          | 16,000 |
| Function return value size | 16 MiB |

## Environment Variables

Applied per-deployment.

|                     |               |
| ------------------- | ------------- |
| Number of variables | 100           |
| Maximum name length | 40 characters |
| Maximum value size  | 8 KiB         |

## File Storage

|           | Free/Starter                                                     | Professional                                        | Notes                                                                                                     |
| --------- | ---------------------------------------------------------------- | --------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Storage   | 1 GiB included<br/>(on Starter: $0.033/month per additional GiB) | 100 GiB included<br/>$0.03/month per additional GiB | Includes user files and backups.                                                                          |
| Bandwidth | 1 GiB/month included<br/>(on Starter: $0.33 per additional GiB)  | 50 GiB/month included<br/>$0.30 per additional GiB  | Includes serving user files, accessing user files inside functions, and generating and restoring backups. |

## Full text search

|                          | Value |
| ------------------------ | ----- |
| Search indexes per table | 4     |
| Filters per search index | 16    |
| Terms per search query   | 16    |
| Filters per search query | 8     |
| Maximum term length      | 32 B  |
| Maximum result set       | 1024  |

## Vector search

|                          | Value                    |
| ------------------------ | ------------------------ |
| Vector indexes per table | 4                        |
| Filters per vector index | 16                       |
| Terms per search query   | 16                       |
| Vectors to search by     | 1                        |
| Dimension fields         | 1 (value between 2-4096) |
| Filters per search query | 64                       |
| Maximum term length      | 32 B                     |
| Maximum result set       | 256 (defaults to 10)     |

If any of these limits don't work for you,
[let us know](https://convex.dev/community)!

Please see our [plans and pricing page](https://www.convex.dev/pricing) for
resource limits. After these limits are hit on a free plan, new mutations that
attempt to commit more insertions or updates may fail. Paid plans have no hard
resource limits - they can scale to billions of documents and TBs of storage.
