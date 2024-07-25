---
name: Convex
title: Convex destination connector for Fivetran
description: Documentation and setup guide for the Convex destination connector for Fivetran
---

# Convex {% typeBadge connector="convex_destination" /%} {% availabilityBadge connector="convex_destination" /%}

[Convex](https://convex.dev) is an all-in-one backend platform with thoughtful, product-centric APIs for application builders.

Note that Convex can also be set up as a [source](/docs/databases/convex)

---

## Setup guide

Follow our [step-by-step Convex setup guide](/docs/destinations/convex_destination/setup-guide) to connect Convex as a destination with Fivetran.

---

## Sync overview

Once Fivetran is connected to your Convex destination, the connector will attempt to sync your data.
It may ask you to update your `convex/schema.ts` in your deployment to match the format of your source.
Once the `convex/schema.ts` matches, data will continue to sync.

---

## Configuration

You will need your deployment URL and deploy key in order to configure the Convex Connector for Fivetran. You can find both on your project's [Production Deployment Settings page](https://docs.convex.dev/dashboard/deployments/deployment-settings).

---

## Schema information

Fivetran tries to replicate the database and columns from your configured source to your destination Convex according to Fivetran's [standard database update strategies](/docs/databases#transformationandmappingoverview).

### Type transformations and mapping

As the connector extracts your data from your source, it matches the supported Fivetran types to [Convex data types](https://docs.convex.dev/database/types).

The following table illustrates how the connector transforms the Fivetran data types into Convex data types.

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

> NOTE: Short/Int are converted to float64 for ease of use in javascript (as `number`). There is no data loss as Number.MAX_SAFE_INTEGER = 2^53 - 1.
> NOTE: Decimal is converted to string to ensure no data loss. Eg "1234.5678"
> NOTE: Naive date uses standard string representation of YYYY-MM-DD.
> NOTE: Naive datetime uses standard string representation of YYYY-MM-DD HH:MM:SS.
> NOTE: UTC datetime uses milliseconds since UNIX epoch

### Fivetran-generated data

Fivetran adds a single `fivetran` column containing a Convex object to the source data.
Some of the fields (`synced`, `deleted`) are for internal purposes.

### Table and column name transformations

If your source table is `default.cars`, then your convex table will be named `default_cars`.
Convex deployments do not have a concept of namespaced tables, so it uses this notation to preserve
the namespace information.

Columns names that begin with `_` are not supported in Convex. Instead, those columns are synced to the
destination nested within the `fivetran.columns` column. For example, a column named `_line` would end up in the `fivetran.columns.line` nested column.
