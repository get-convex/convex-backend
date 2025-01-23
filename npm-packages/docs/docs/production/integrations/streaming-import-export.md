---
title: "Streaming Data in and out of Convex"
sidebar_label: "Streaming Import/Export"
description: "Streaming Data in and out of Convex"
sidebar_position: 4
---

[Fivetran](https://www.fivetran.com) and [Airbyte](https://airbyte.com) are data
integration platforms that allow you to sync your Convex data with other
databases.

Fivetran enables streaming export from Convex to any of their
[supported destinations](https://fivetran.com/docs/destinations). The Convex
team maintains a Convex source connector, for streaming export. Streaming import
into Convex via Fivetran is not supported at the moment.

Using Airbyte enables streaming import from any of their
[supported sources](https://airbyte.com/connectors?connector-type=Sources) into
Convex and streaming export from Convex into any of their
[supported destinations](https://airbyte.com/connectors?connector-type=Destinations).
The Convex team maintains a Convex source connector for streaming export and a
Convex destination connector for streaming import.

<BetaAdmonition feature="Fivetran & Airbyte integrations" verb="are" />

## Streaming Export

Exporting data can be useful for handling workloads that aren't supported by
Convex directly. Some use cases include:

1. Analytics
   - Convex isn't optimized for queries that load huge amounts of data. A data
     platform like [Databricks](https://www.databricks.com) or
     [Snowflake](https://www.snowflake.com/) is more appropriate.
2. Flexible querying
   - While Convex has powerful
     [database queries](/docs/database/reading-data/reading-data.mdx#querying-documents)
     and built-in [full text search](/docs/search.mdx) support, there are still
     some queries that are difficult to write within Convex. If you need very
     dynamic sorting and filtering for something like an "advanced search" view,
     databases like [ElasticSearch](https://www.elastic.co) can be helpful.
3. Machine learning training
   - Convex isn't optimized for queries running computationally intensive
     machine learning algorithms.

<ProFeatureUpsell feature="Streaming export" verb="requires" />

See the [Fivetran](https://fivetran.com/integrations/convex) or
[Airbyte](https://docs.airbyte.com/integrations/sources/convex) docs to learn
how to set up a streaming export. [Contact us](https://convex.dev/community) if
you need help or have questions.

## Streaming Import

Adopting new technologies can be a slow, daunting process, especially when the
technologies involve databases. Streaming import enables adopting Convex
alongside your existing stack without having to write your own migration or data
sync tooling. Some use cases include:

1. Prototyping how Convex could replace your project's existing backend using
   its own data.
2. Building new products faster by using Convex alongside existing databases.
3. Developing a reactive UI-layer on top of an existing dataset.
4. Migrating your data to Convex (if the [CLI](/docs/cli.md) tool doesn't meet
   your needs).

<Admonition type="caution" title="Make imported tables read-only">
A common use case is to "mirror" a table in the source database to Convex to
build something new using Convex. We recommend leaving imported
tables as read-only in Convex because syncing the results back to the source
database could result in dangerous write conflicts. While Convex doesn't yet
have access controls that would ensure a table is read-only, you can make sure that
there are no mutations or actions writing to imported tables in your code and avoid editing
documents in imported tables in the dashboard.
</Admonition>

Streaming import is included with all Convex plans. See the Airbyte docs on how
to set up the Convex destination connector
[here](https://docs.airbyte.com/integrations/destinations/convex).
