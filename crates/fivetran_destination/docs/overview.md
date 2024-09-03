---
name: Convex
title: Fivetran for Convex | Configuration and documentation
description: Connect data sources to Convex with our partner-built Convex destination connector. Explore documentation and start syncing your applications, databases, events, files, and more.
menuPosition: 55
hidden: true
---

# Convex {% typeBadge connector="convex_destination" /%} {% availabilityBadge connector="convex_destination" /%}

[Convex](https://convex.dev) is a full-stack TypeScript development platform with product-centric APIs. It can replace your database and server functions.

> NOTE: Fivetran supports Convex as both a partner-built [database connector](/docs/databases/convex) and a destination.

> NOTE: This destination is [partner-built](/docs/partner-built-program). For any questions related to the Convex destination and its documentation, refer to Convex's support team. For SLA details, see [Convex's Status and Guarantees documentation](https://docs.convex.dev/production/state).

---

## Setup guide

Follow our [step-by-step Convex setup guide](/docs/destinations/convex/setup-guide) to connect Convex as a destination with Fivetran.

---

## Schema information

Fivetran tries to replicate the database and columns from your data source to your Convex destination according to Fivetran's [standard database update strategies](/docs/databases#transformationandmappingoverview).

Once Fivetran connects to your Convex destination, the connector will attempt to load your data.
It may ask you to update your `convex/schema.ts` in your destination to match the format of your source.
Once the `convex/schema.ts` matches the source format, data will continue to sync.

### Type transformation mapping

The Convex destination extracts data from your source, and it matches supported [Fivetran data types](/docs/destinations#datatypes) to [Convex data types](https://docs.convex.dev/database/types).

We use the following data type conversions:

| Fivetran Type | Convex Type | Equivalence |
| ------------- | ----------- | ----------- |
| BOOLEAN       | Boolean     | Exact       |
| SHORT         | Float64     | Inexact     |
| INT           | Float64     | Inexact     |
| LONG          | Int64       | Exact       |
| DECIMAL       | String      | Inexact     |
| FLOAT         | Float64     | Inexact     |
| DOUBLE        | Float64     | Exact       |
| NAIVEDATE     | String      | Inexact     |
| NAIVEDATETIME | String      | Inexact     |
| UTCDATETIME   | Float64     | Inexact     |
| BINARY        | Bytes       | Exact       |
| STRING        | String      | Exact       |
| NULL          | Null        | Exact       |
| JSON          | Object      | Inexact     |

> NOTE: Short/Int are converted to float64 for ease of use in javascript (as `number`). There is no data loss as `Number.MAX_SAFE_INTEGER = 2^53 - 1`. Decimal is converted to STRING to ensure no data loss (e.g. "1234.5678").

> NOTE: Naive date uses standard string representation of `YYYY-MM-DD`. Naive datetime uses standard string representation of `YYYY-MM-DD HH:MM:SS`.

> NOTE: UTC datetime uses milliseconds since UNIX epoch.

### Fivetran-generated data

Fivetran adds a single `fivetran` column containing a Convex object to the source data.
Some of the columns (`synced`, `deleted`) are for internal purposes.

### Table and column name transformations

If your source table is `default.cars`, then your Convex table will be named `default_cars`.
Convex deployments do not have a concept of namespaced tables, so it uses this notation to preserve
the namespace information.

Column names that begin with `_` are not supported in Convex. Instead, those columns are synced to the
destination nested within the `fivetran.columns` column. For example, a column named `_line` would be synced as a nested column named `fivetran.columns.line`.