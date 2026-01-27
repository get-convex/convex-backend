---
title: "索引"
sidebar_position: 100
description: "使用数据库索引加速查询"
---

索引是一种数据结构，可以通过告诉 Convex 如何组织文档来加速
[文档查询](/database/reading-data/reading-data.mdx#querying-documents)。
索引还允许你更改查询结果中文档的顺序。

如需更深入地了解索引，请参阅
[索引与查询性能](/database/reading-data/indexes/indexes-and-query-perf.md)。

## 定义索引 \{#defining-indexes\}

索引作为 Convex [模式](/database/schemas.mdx) 的一部分进行定义。每个索引由以下内容组成：

1. 名称。
   * 在每张表中必须唯一。
2. 要建立索引的字段的有序列表。
   * 要指定嵌套文档中的字段，使用以点分隔的路径，例如
     `properties.name`。

要在表上添加索引，在该表的模式上使用
[`index`](/api/classes/server.TableDefinition#index) 方法：

```ts noDialect title="convex/schema.ts"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

// 定义一个带有两个索引的 messages 表。
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

`by_channel` 索引按照模式中定义的 `channel` 字段排序。
对于同一频道中的消息，它们会按照
[系统生成的 `_creationTime` 字段](/database/types.md#system-fields) 排序，
该字段会自动添加到所有索引中。

相比之下，`by_channel_user` 索引会将同一 `channel` 中的消息先按发送它们的
`user` 排序，然后再按 `_creationTime` 排序。

在运行 [`npx convex dev`](/cli.md#run-the-convex-dev-server) 和
[`npx convex deploy`](/cli.md#deploy-convex-functions-to-production) 时会创建索引。

你可能会注意到，第一次定义某个索引的部署会比平时慢一些。
这是因为 Convex 需要对你的索引进行 *回填（backfill）*。
表中的数据越多，Convex 按索引顺序整理这些数据所花费的时间就越长。
如果你需要给大型表添加索引，请使用[分阶段索引](#staged-indexes)。

你可以在定义索引的同一次部署中放心地查询该索引。
Convex 会确保在注册新的查询和变更函数之前完成索引的回填。

<Admonition type="caution" title="删除索引时要小心">
  除了添加新索引之外，`npx convex deploy` 还会删除在你的模式中不再存在的索引。
  在从模式中删除索引之前，请确保这些索引已经完全不再使用！
</Admonition>

## 使用索引查询文档 \{#querying-documents-using-indexes\}

针对 `by_channel` 索引，“查询在 `channel` 中于 1–2 分钟前创建的消息”可以这样写：

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

[`.withIndex`](/api/interfaces/server.QueryInitializer#withindex) 方法
定义要查询的索引，以及 Convex 将如何使用该索引来选择文档。第一个参数是索引名称，第二个参数是
*索引范围表达式（index range expression）*。索引范围表达式用于描述 Convex 在运行查询时
应当考虑哪些文档。

索引的选择既会影响索引范围表达式的写法，也会影响返回结果的顺序。例如，通过同时创建
`by_channel` 和 `by_channel_user` 两个索引，我们可以在同一个 channel 内获取按 `_creationTime`
或者按 `user` 排序的结果。如果你像下面这样使用 `by_channel_user` 索引：

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel_user", (q) => q.eq("channel", channel))
  .collect();
```

结果会是某个 `channel` 中的所有消息，先按 `user` 排序，
再按 `_creationTime` 排序。如果你像下面这样使用 `by_channel_user`：

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel_user", (q) =>
    q.eq("channel", channel).eq("user", user),
  )
  .collect();
```

结果将会是给定 `channel` 中由 `user` 发送的消息，并按 `_creationTime` 排序。

索引范围表达式始终是一个按顺序链式调用的列表：

1. 0 个或多个使用
   [`.eq`](/api/interfaces/server.IndexRangeBuilder#eq) 定义的等值表达式。
2. [可选] 一个使用
   [`.gt`](/api/interfaces/server.IndexRangeBuilder#gt) 或
   [`.gte`](/api/interfaces/server.IndexRangeBuilder#gte) 定义的下界表达式。
3. [可选] 一个使用
   [`.lt`](/api/interfaces/server.IndexRangeBuilder#lt) 或
   [`.lte`](/api/interfaces/server.IndexRangeBuilder#lte) 定义的上界表达式。

**你必须按照索引字段的顺序依次处理每个字段。**

每个等值表达式必须比较一个不同的索引字段，从第一个字段开始并保持顺序。上界和下界表达式必须放在所有等值表达式之后，并比较下一个字段。

例如，不可能写出如下这样的查询：

```ts
// 无法编译!
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel", (q) =>
    q
      .gt("_creationTime", Date.now() - 2 * 60000)
      .lt("_creationTime", Date.now() - 60000),
  )
  .collect();
```

这个查询无效，因为 `by_channel` 索引是按 `(channel, _creationTime)` 排序的，而该查询范围在 `_creationTime` 上做了比较，却没有先将范围限定到某一个 `channel`。
由于索引先按 `channel` 排序，再按 `_creationTime` 排序，因此它并不适合用来查找所有频道中在 1–2 分钟前创建的消息。
`withIndex` 中的 TypeScript 类型会引导你完成这类操作。

要更好地理解哪些查询可以在哪些索引上运行，请参阅
[索引简介与查询性能](/database/reading-data/indexes/indexes-and-query-perf.md)。

**查询的性能取决于范围限定得有多具体。**

例如，如果查询是

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

那么查询的性能将取决于 `channel` 中在过去 1–2 分钟内创建的消息数量。

如果未指定索引范围，查询将会考虑该索引中的所有文档。

<Admonition type="tip" title="选择一个好的索引范围">
  为了获得更好的性能，请定义尽可能具体的索引范围！如果你在查询一张大表，
  却无法通过 `.eq` 添加任何相等条件，就应该考虑定义一个新的索引。
</Admonition>

`.withIndex` 的设计目标是只允许你指定那些 Convex 能够高效利用索引来查找的范围。
对于其他所有的筛选，你都可以使用 [`.filter`](/api/interfaces/server.Query#filter) 方法。

例如，要查询“在 `channel` 中**不是**由我创建的消息”，你可以这样做：

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel", (q) => q.eq("channel", channel))
  .filter((q) => q.neq(q.field("user"), myUserId))
  .collect();
```

在这种情况下，该查询的性能将取决于频道中的消息数量。Convex 会检查频道中的每条消息，只返回其中 `user` 字段不等于 `myUserId` 的消息。

## 使用索引进行排序 \{#sorting-with-indexes\}

使用 `withIndex` 的查询会按照索引中指定的列进行排序。

索引中列的顺序决定了排序优先级。会先比较索引中最前面的列的值。
只有在所有更早的列都相等时，后续列才会作为决胜字段参与比较。

由于 Convex 会自动把 `_creationTime` 作为最后一列加入到所有索引中，
如果索引中的其他列都相等，`_creationTime` 将始终作为最终的决胜字段。

例如，`by_channel_user` 包含 `channel`、`user` 和 `_creationTime`。
因此，对 `messages` 使用 `.withIndex("by_channel_user")` 的查询会先按频道排序，
然后在每个频道内按用户排序，最后按创建时间排序。

使用索引排序可以满足诸如展示前 `N` 个得分最高的用户、最近的 `N` 条交易记录，或被点赞最多的 `N` 条消息等用例。

例如，要获取游戏中得分最高的前 10 名玩家，你可以在玩家的最高得分上定义一个索引：

```ts
export default defineSchema({
  players: defineTable({
    username: v.string(),
    highestScore: v.number(),
  }).index("by_highest_score", ["highestScore"]),
});
```

然后，你就可以使用该索引和 [`take(10)`](/api/interfaces/server.Query#take) 来高效查找得分最高的前 10 名玩家：

```ts
const topScoringPlayers = await ctx.db
  .query("users")
  .withIndex("by_highest_score")
  .order("desc")
  .take(10);
```

在这个示例中，我们省略了范围表达式，因为我们要查找的是有史以来得分最高的玩家。这个特定查询在数据量很大的情况下仍然相当高效，主要是因为我们使用了 `take()`。

如果在不使用范围表达式的情况下使用索引，你应当始终在 `withIndex` 后配合使用下列方法之一：

1. [`.first()`](/api/interfaces/server.Query#first)
2. [`.unique()`](/api/interfaces/server.Query#unique)
3. [`.take(n)`](/api/interfaces/server.Query#take)
4. [`.paginate(ops)`](/database/pagination.mdx)

这些 API 允许你在不执行全表扫描的情况下，高效地将查询结果限制在一个合理的规模内。

<Admonition type="caution" title="全表扫描">
  当你的查询从数据库中获取文档时，它会扫描你指定范围内的行。比如，如果你使用 `.collect()`，它会扫描该范围内的所有行。因此，如果你在没有范围表达式的情况下使用 `withIndex`，你将会
  [扫描整个表](https://docs.convex.dev/database/indexes/indexes-and-query-perf#full-table-scans)，
  当你的表有成千上万行时，这会很慢。`.filter()` 不会改变将要扫描的文档范围。使用 `.first()`、`.unique()` 或
  `.take(n)` 只会扫描到获取到足够数量的文档为止。
</Admonition>

你可以加入一个范围表达式，以满足更有针对性的查询。比如，如果要获取加拿大得分最高的玩家，你可以同时使用 `take()`
和范围表达式：

```ts
// 查询加拿大得分最高的前 10 名玩家。
const topScoringPlayers = await ctx.db
  .query("users")
  .withIndex("by_country_highest_score", (q) => q.eq("country", "CA"))
  .order("desc")
  .take(10);
```

## 分阶段索引 \{#staged-indexes\}

默认情况下，你在部署代码时会同步创建索引。对于大型表，为现有表
[回填索引](indexes-and-query-perf#backfilling-and-maintaining-indexes)
的过程可能会比较慢。分阶段索引是一种在大型表上异步创建索引且不会阻塞部署流程的方式。如果你同时在开发多个功能，这会很有用。

要创建分阶段索引，请在你的 `schema.ts` 中使用以下语法。

```ts
export default defineSchema({
  messages: defineTable({
    channel: v.id("channels"),
  }).index("by_channel", { fields: ["channel"], staged: true }),
});
```

<Admonition type="caution" title="暂存索引在启用前无法使用">
  暂存索引在你启用之前无法在查询中使用。要启用它们，
  必须先完成回填。
</Admonition>

你可以在仪表盘的数据页面的
[*Indexes* 面板](/dashboard/deployments/data/#view-the-indexes-of-a-table)
中查看回填进度。回填完成后，你可以启用该索引，并通过移除
`staged` 选项来使用它。

```ts
export default defineSchema({
  messages: defineTable({
    channel: v.id("channels"),
  }).index("by_channel", { fields: ["channel"] }),
});
```

## 限制 \{#limits\}

Convex 支持最多包含 16 个字段的索引。你可以在每个表上定义 32 个索引。索引中不能包含重复字段。

索引中不允许使用保留字段（以下划线 `_` 开头）。`_creationTime` 字段会自动添加到每个索引的末尾，以保证稳定的排序。它不应在索引定义中被显式添加，并且会计入索引字段数量上限。

`by_creation_time` 索引会自动创建（并且会在未指定索引的数据库查询中使用）。`by_id` 索引是保留索引。