---
title: "Indexes"
sidebar_position: 100
---

Indexes are a data structure that allow you to speed up your
[document queries](/docs/database/reading-data/reading-data.mdx#querying-documents)
by telling Convex how to organize your documents. Indexes also allow you to
change the order of documents in query results.

For a more in-depth introduction to indexing see
[Indexes and Query Performance](/docs/database/reading-data/indexes/indexes-and-query-perf.md).

## Defining indexes

Indexes are defined as part of your Convex [schema](/docs/database/schemas.mdx).
Each index consists of:

1. A name.
   - Must be unique per table.
2. An ordered list of fields to index.
   - To specify a field on a nested document, use a dot-separated path like
     `properties.name`.

To add an index onto a table, use the
[`index`](/api/classes/server.TableDefinition#index) method on your table's
schema:

```ts noDialect title="convex/schema.ts"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

// Define a messages table with two indexes.
export default defineSchema({
  messages: defineTable({
    channel: v.id("channels"),
    body: v.string(),
    user: v.id("users"),
  })
    .index("by_channel", ["channel"])
    .index("by_channel_user", ["channel", "user"]),
});
```

The `by_channel` index is ordered by the `channel` field defined in the schema.
For messages in the same channel, they are ordered by the
[system-generated `_creationTime` field](/docs/database/types.md#system-fields)
which is added to all indexes automatically.

By contrast, the `by_channel_user` index orders messages in the same `channel`
by the `user` who sent them, and only then by `_creationTime`.

Indexes are created in
[`npx convex dev`](/docs/cli.md#run-the-convex-dev-server) and
[`npx convex deploy`](/docs/cli.md#deploy-convex-functions-to-production).

You may notice that the first deploy that defines an index is a bit slower than
normal. This is because Convex needs to _backfill_ your index. The more data in
your table, the longer it will take Convex to organize it in index order. If
this is problematic for your workflow,
[contact us](/docs/production/contact.md).

You can feel free to query an index in the same deploy that defines it. Convex
will ensure that the index is backfilled before the new query and mutation
functions are registered.

<Admonition type="caution" title="Be careful when removing indexes">

In addition to adding new indexes, `npx convex deploy` will delete indexes that
are no longer present in your schema. Make sure that your indexes are completely
unused before removing them from your schema!

</Admonition>

## Querying documents using indexes

A query for "messages in `channel` created 1-2 minutes ago" over the
`by_channel` index would look like:

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel", (q) =>
    q
      .eq("channel", channel)
      .gt("_creationTime", Date.now() - 2 * 60000)
      .lt("_creationTime", Date.now() - 60000),
  )
  .collect();
```

The [`.withIndex`](/api/interfaces/server.QueryInitializer#withindex) method
defines which index to query and how Convex will use that index to select
documents. The first argument is the name of the index and the second is an
_index range expression_. An index range expression is a description of which
documents Convex should consider when running the query.

The choice of index both affects how you write the index range expression and
what order the results are returned in. For instance, by making both a
`by_channel` and `by_channel_user` index, we can get results within a channel
ordered by `_creationTime` or by `user`, respectively. If you were to use the
`by_channel_user` index like this:

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel_user", (q) => q.eq("channel", channel))
  .collect();
```

The results would be all of the messages in a `channel` ordered by `user`, then
by `_creationTime`. If you were to use `by_channel_user` like this:

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel_user", (q) =>
    q.eq("channel", channel).eq("user", user),
  )
  .collect();
```

The results would be the messages in the given `channel` sent by `user`, ordered
by `_creationTime`.

An index range expression is always a chained list of:

1. 0 or more equality expressions defined with
   [`.eq`](/api/interfaces/server.IndexRangeBuilder#eq).
2. [Optionally] A lower bound expression defined with
   [`.gt`](/api/interfaces/server.IndexRangeBuilder#gt) or
   [`.gte`](/api/interfaces/server.IndexRangeBuilder#gte).
3. [Optionally] An upper bound expression defined with
   [`.lt`](/api/interfaces/server.IndexRangeBuilder#lt) or
   [`.lte`](/api/interfaces/server.IndexRangeBuilder#lte).

**You must step through fields in index order.**

Each equality expression must compare a different index field, starting from the
beginning and in order. The upper and lower bounds must follow the equality
expressions and compare the next field.

For example, it is not possible to write a query like:

```ts
// DOES NOT COMPILE!
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel", (q) =>
    q
      .gt("_creationTime", Date.now() - 2 * 60000)
      .lt("_creationTime", Date.now() - 60000),
  )
  .collect();
```

This query is invalid because the `by_channel` index is ordered by
`(channel, _creationTime)` and this query range has a comparison on
`_creationTime` without first restricting the range to a single `channel`.
Because the index is sorted first by `channel` and then by `_creationTime`, it
isn't a useful index for finding messages in all channels created 1-2 minutes
ago. The TypeScript types within `withIndex` will guide you through this.

To better understand what queries can be run over which indexes, see
[Introduction to Indexes and Query Performance](/docs/database/reading-data/indexes/indexes-and-query-perf.md).

**The performance of your query is based on the specificity of the range.**

For example, if the query is

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel", (q) =>
    q
      .eq("channel", channel)
      .gt("_creationTime", Date.now() - 2 * 60000)
      .lt("_creationTime", Date.now() - 60000),
  )
  .collect();
```

then query's performance would be based on the number of messages in `channel`
created 1-2 minutes ago.

If the index range is not specified, all documents in the index will be
considered in the query.

<Admonition type="tip" title="Picking a good index range">

For performance, define index ranges that are as specific as possible! If you
are querying a large table and you're unable to add any equality conditions with
`.eq`, you should consider defining a new index.

</Admonition>

`.withIndex` is designed to only allow you to specify ranges that Convex can
efficiently use your index to find. For all other filtering you can use the
[`.filter`](/api/interfaces/server.Query#filter) method.

For example to query for "messages in `channel` **not** created by me" you could
do:

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel", q => q.eq("channel", channel))
  .filter(q => q.neq(q.field("user"), myUserId)
  .collect();
```

In this case the performance of this query will be based on how many messages
are in the channel. Convex will consider each message in the channel and only
return the messages where the `user` field matches `myUserId`.

## Sorting with indexes

Queries that use `withIndex` are ordered by the columns specified in the index.

The order of the columns in the index dictates the priority for sorting. The
values of the columns listed first in the index are compared first. Subsequent
columns are only compared as tie breakers only if all earlier columns match.

Since Convex automatically includes `_creationTime` as the last column in all
indexes, `_creationTime` will always be the final tie breaker if all other
columns in the index are equal.

For example, `by_channel_user` includes `channel`, `user`, and `\_creationTime`.
So queries on `messages` that use `.withIndex("by_channel_user")` will be sorted
first by channel, then by user within each channel, and finally by the creation
time.

Sorting with indexes allows you to satisfy use cases like displaying the top `N`
scoring users, the most recent `N` transactions, or the most `N` liked messages.

For example, to get the top 10 highest scoring players in your game, you might
define an index on the player's highest score:

```ts
export default defineSchema({
  players: defineTable({
    username: v.string(),
    highestScore: v.number(),
  }).index("by_highest_score", ["highestScore"]),
});
```

You can then efficiently find the top 10 highest scoring players using your
index and [`take(10)`](/api/interfaces/server.Query#take):

```ts
const topScoringPlayers = await ctx.db
  .query("users")
  .withIndex("by_highest_score")
  .order("desc")
  .take(10);
```

In this example, the range expression is omitted because we're looking for the
highest scoring players of all time. This particular query is reasonably
efficient for large data sets only because we're using `take()`.

If you use an index without a range expression, you should always use one of the
following in conjunction with `withIndex`:

1. [`.first()`](/api/interfaces/server.Query#first)
2. [`.unique()`](/api/interfaces/server.Query#unique)
3. [`.take(n)`](/api/interfaces/server.Query#take)
4. [`.paginate(ops)`](/docs/database/pagination.mdx)

These APIs allow you to efficiently limit your query to a reasonable size
without performing a full table scan.

<Admonition type="caution" title="Full Table Scans">

When your query fetches documents from the database, it will scan the rows in
the range you specify. If you are using `.collect()`, for instance, it will scan
all of the rows in the range. So if you use `withIndex` without a range
expression, you will be
[scanning the whole table](https://docs.convex.dev/database/indexes/indexes-and-query-perf#full-table-scans),
which can be slow when your table has thousands of rows. `.filter()` doesn't
affect which documents are scanned. Using `.first()` or `.unique()` or
`.take(n)` will only scan rows until it has enough documents.

</Admonition>

You can include a range expression to satisfy more targeted queries. For
example, to get the top scoring players in Canada, you might use both `take()`
and a range expression:

```ts
// query the top 10 highest scoring players in Canada.
const topScoringPlayers = await ctx.db
  .query("users")
  .withIndex("by_country_highest_score", (q) => q.eq("country", "CA"))
  .order("desc")
  .take(10);
```

## Limits

Convex supports indexes containing up to 16 fields. You can define 32 indexes on
each table. Indexes can't contain duplicate fields.

No reserved fields (starting with `_`) are allowed in indexes. The
`_creationTime` field is automatically added to the end of every index to ensure
a stable ordering. It should not be added explicitly in the index definition,
and it's counted towards the index fields limit.

The `by_creation_time` index is created automatically (and is what is used in
database queries that don't specify an index). The `by_id` index is reserved.
