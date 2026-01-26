---
sidebar_label: "索引与查询性能"
title: "索引与查询性能简介"
sidebar_position: 100
description: "了解索引对查询性能的影响"
---

如何确保我的 Convex
[数据库查询](/database/reading-data/reading-data.mdx) 运行快速且高效？我应该在什么时候定义
[索引](/database/reading-data/indexes/indexes.md)？什么是索引？

本文档通过描述一个简化模型，来说明 Convex 中查询和索引是如何工作的，从而帮助你思考和优化查询性能。

如果你已经对数据库查询和索引有了较深入的理解，可以直接查看参考文档：

* [读取数据](/database/reading-data/reading-data.mdx)
* [索引](/database/reading-data/indexes/indexes.md)

## 文档的图书馆 \{#a-library-of-documents\}

你可以把 Convex 想象成一个实体图书馆，把文档以纸质书的形式存放。在这个世界里，每次你通过
[`db.insert("books", {...})`](/api/interfaces/server.GenericDatabaseWriter#insert)
向 Convex 添加一个文档时，图书管理员就会把这本书放到书架上。

默认情况下，Convex 会按照文档被插入的顺序来组织它们。你可以想象图书管理员在书架上从左到右地把文档放上去。

如果你运行一个查询来查找第一本书，例如：

```ts
const firstBook = await ctx.db.query("books").first();
```

那么图书管理员可以从书架的最左端开始，找到第一本书。这个查询非常快，因为图书管理员只需要看一本书就能得到结果。

类似地，如果我们想要获取最后一本被插入的书，我们可以改为这样做：

```ts
const lastBook = await ctx.db.query("books").order("desc").first();
```

这是同一个查询，但我们把顺序改成了降序。在图书馆这个比喻中，这意味着图书管理员会从书架的最右端开始，从右向左扫描。图书管理员仍然只需要看一本书就能确定结果，因此这个查询同样非常快。

## 全表扫描 \{#full-table-scans\}

现在想象一下，有人走进图书馆，问：“你们有 Jane Austen 写的哪些书？”

这个查询可以表示为：

```ts
const books = await ctx.db
  .query("books") // 查询 books 表
  .filter((q) => q.eq(q.field("author"), "Jane Austen"))
  .collect();
```

这个查询的意思是：“从左到右查看所有书籍，收集那些 `author` 字段为 Jane Austen 的书。” 为了做到这一点，图书管理员需要查看整排书架，并检查每一本书的作者信息。

这个查询是一次 *全表扫描*，因为它需要 Convex 查看表中的每一个文档。这个查询的性能取决于图书馆里的藏书数量。

如果你的 Convex 表中文档数量很少，这没有问题！即使有几百个文档，全表扫描通常也很快，但如果这张表里有成千上万个文档，这类查询就会变慢。

在图书馆的类比中，如果图书馆只有一个书架，这种查询方式是可以接受的。随着图书馆扩展成有许多书架或许多书柜的规模，这种方法就变得不可行了。

## 卡片目录 \{#card-catalogs\}

我们如何才能更高效地按作者找到书？

一种做法是按 `author` 重新为整个图书馆排序。这样能解决我们当前的问题，
但此时我们最初对 `firstBook` 和 `lastBook` 的查询会变成整表扫描，
因为我们需要检查每一本书来确定哪一本是最先/最后被放入的。

另一种做法是复制整个馆藏。我们可以为每本书购买 2 本，并把它们放在 2 个不同的书架上：
一个书架按放入时间排序，另一个按作者排序。这样也行，但代价很高。
我们现在需要为图书馆准备两倍的空间。

更好的做法是在 `author` 上建立一个&#95;索引&#95;。在图书馆中，我们可以使用老式的
[卡片目录](https://en.wikipedia.org/wiki/Library_catalog) 来按作者组织书籍。
其思路是，图书管理员为每本书写一张索引卡片，卡片上包含：

* 书的作者
* 书在书架上的位置

这些索引卡会按作者排序，并保存在与放置书籍的书架分开的整理柜中。
卡片目录应该保持小巧，因为它只为每本书保存一张索引卡（而不是整本书的全文）。

![Card Catalog](/img/card-catalog.jpg)

当一位读者说“我想找 Jane Austen 的书”时，图书管理员现在可以：

1. 去卡片目录中快速找到所有 “Jane Austen” 的卡片。
2. 对于每张卡片，到书架上找到对应的书。

这非常快，因为图书管理员可以迅速找到 Jane Austen 的索引卡。
为每张卡片找到对应的书还需要做一点工作，但索引卡的数量很少，所以整体仍然非常快。

## 索引 \{#indexes\}

数据库索引也是基于同样的概念工作的！在 Convex 中，你可以这样定义一个
*索引*：

```ts noDialect title="convex/schema.ts"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  books: defineTable({
    author: v.string(),
    title: v.string(),
    text: v.string(),
  }).index("by_author", ["author"]),
});
```

那么 Convex 会在 `author` 上创建一个名为 `by_author` 的新索引。这意味着
你的 `books` 表现在会额外多出一个按照 `author` 字段排序的
数据结构。

你可以通过以下方式查询这个索引：

```ts
const austenBooks = await ctx.db
  .query("books")
  .withIndex("by_author", (q) => q.eq("author", "Jane Austen"))
  .collect();
```

这个查询指示 Convex 使用 `by_author` 索引，找到所有满足
`doc.author === "Jane Austen"` 的条目。因为这个索引是按 `author` 排序的，
所以这是一个非常高效的操作。这意味着 Convex 可以以与图书管理员相同的方式来执行
这个查询：

1. 找到索引中包含 Jane Austen 条目的那一段区间。
2. 对该区间内的每个条目，获取对应的文档。

这个查询的性能取决于满足
`doc.author === "Jane Austen"` 的文档数量，而这个数量应该非常小。我们已经极大地
提升了这个查询的速度！

## 回填和维护索引 \{#backfilling-and-maintaining-indexes\}

一个值得思考的有趣细节，是创建这种新结构所需要的工作量。在图书馆里，图书管理员必须逐本查看书架上的每一本书，并在卡片目录中为每一本书新建一张按作者排序的索引卡。只有在完成这些工作之后，图书管理员才能放心地依赖卡片目录给出正确的结果。

Convex 索引也是一样的！当你定义了一个新索引，第一次运行 `npx convex deploy` 时，Convex 需要遍历你的所有文档并为每一条文档建立索引。这就是为什么在创建新索引后的第一次部署会比平时稍慢一些；Convex 必须为表中的每一条文档做一些额外工作。如果这个表特别大，可以考虑使用[分阶段索引](/database/reading-data/indexes#staged-indexes)，在部署流程之外异步完成回填。

同样地，即使索引已经定义，随着数据变化，Convex 仍然需要做一点额外工作来保持索引的最新状态。每当在一个已索引的表中插入、更新或删除文档时，Convex 也会更新它在索引中的条目。这类似于图书管理员在往图书馆新增图书时，为新书创建新的索引卡。

如果你只定义了少量索引，就不需要担心维护成本。随着你定义的索引越来越多，维护它们的成本也会增加，因为每一次 `insert` 操作都需要更新每一个索引。这就是为什么 Convex 将每个表的索引数量限制为 32 个。在实践中，大多数应用会为每个表定义少量索引，以让关键查询保持高效。

## 多字段索引 \{#indexing-multiple-fields\}

现在想象一位读者来到图书馆，想要借出 Isaac Asimov 的 *Foundation*。由于我们在 `author` 字段上已经建立了索引，我们可以写一个查询，利用该索引找到所有 Isaac Asimov 的书，然后检查每本书的标题，判断是否是 *Foundation*。

```ts
const foundation = await ctx.db
  .query("books")
  .withIndex("by_author", (q) => q.eq("author", "Isaac Asimov"))
  .filter((q) => q.eq(q.field("title"), "Foundation"))
  .unique();
```

这个查询描述了图书管理员可能如何执行这个查询。图书管理员会
使用卡片目录来找到 Isaac Asimov 所有书籍的索引卡片。
这些卡片本身并不包含书名，所以图书管理员需要在书架上找到
每一本 Asimov 的书，并查看其标题，以找到那本名为 *Foundation* 的书。
最后，这个查询以
[`.unique`](/api/interfaces/server.Query#unique) 结束，因为我们预期
结果最多只有一条。

这个查询演示了使用
[`withIndex`](/api/interfaces/server.QueryInitializer#withindex) 和
[`filter`](/api/interfaces/server.Query#filter) 进行过滤的区别。
`withIndex` 只允许你基于索引来限制查询。你只能执行索引能高效完成的操作，
比如查找具有指定作者的所有文档。

另一方面，`filter` 允许你编写任意、复杂的表达式，
但它不会使用索引来运行。相反，`filter` 表达式会
对范围内的每一个文档进行求值。

综上，我们可以得出结论：**已建立索引的查询性能取决于索引范围中有多少文档**。
在这个例子中，性能取决于 Isaac Asimov 书的数量，因为图书管理员
需要查看每一本书的标题。

不幸的是，Isaac Asimov 写了
[很多书](https://en.wikipedia.org/wiki/Isaac_Asimov_bibliography_\(alphabetical\))。
在现实中，即使有 500 多本书，在 Convex 上使用现有索引依然会足够快，
但我们还是来考虑一下如何改进它。

一种做法是基于 `title` 构建一个单独的 `by_title` 索引。
这样我们就可以把现在在 `.filter` 和 `.withIndex` 中所做的工作对调，变成：

```ts
const foundation = await ctx.db
  .query("books")
  .withIndex("by_title", (q) => q.eq("title", "Foundation"))
  .filter((q) => q.eq(q.field("author"), "Isaac Asimov"))
  .unique();
```

在这个查询中，我们高效地利用索引先找到所有名为 *Foundation* 的书，然后再在这些结果中筛选出 Isaac Asimov 所写的那一本。

这样做还行，但我们仍然有可能因为太多书的标题都是 *Foundation* 而导致该查询变慢。一个更好的方法是构建一个*复合*索引，同时索引 `author` 和 `title`。复合索引是建立在一个有序字段列表上的索引。

```ts noDialect title="convex/schema.ts"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  books: defineTable({
    author: v.string(),
    title: v.string(),
    text: v.string(),
  }).index("by_author_title", ["author", "title"]),
});
```

在这个索引中，书籍首先按作者排序，然后在每位作者名下再按书名排序。也就是说，图书管理员可以利用这个索引跳转到 Isaac Asimov 的部分，并在其中快速找到 *Foundation*。

如果用 Convex 查询来表示，就是这样的：

```ts
const foundation = await ctx.db
  .query("books")
  .withIndex("by_author_title", (q) =>
    q.eq("author", "Isaac Asimov").eq("title", "Foundation"),
  )
  .unique();
```

这里的索引范围表达式告诉 Convex 只考虑 `author` 为 Isaac Asimov 且 `title` 为 *Foundation* 的文档。由于这只是一个文档，因此这个查询会非常快！

因为这个索引先按 `author` 再按 `title` 排序，它也能高效地支持类似“所有 Isaac Asimov 写的、标题以 F 开头的书”这样的查询。我们可以这样来表示：

```ts
const asimovBooksStartingWithF = await ctx.db
  .query("books")
  .withIndex("by_author_title", (q) =>
    q.eq("author", "Isaac Asimov").gte("title", "F").lt("title", "G"),
  )
  .collect();
```

This query uses the index to find books where
`author === "Isaac Asimov" && "F" <= title < "G"`. Once again, the performance
of this query is based on how many documents are in the index range. In this
case, that&#39;s just the Asimov books that begin with &quot;F&quot; which is quite small.

Also note that this index also supports our original query for &quot;books by Jane
Austen.&quot; It&#39;s okay to only use the `author` field in an index range expression
and not restrict by title at all.

Lastly, imagine that a library patron asks for the book *The Three-Body Problem*
but they don&#39;t know the author&#39;s name. Our `by_author_title` index won&#39;t help us
here because it&#39;s sorted first by `author`, and then by `title`. The title, *The
Three-Body Problem*, could appear anywhere in the index!

The Convex TypeScript types in the `withIndex` make this clear because they
require that you compare index fields in order. Because the index is defined on
`["author", "title"]`, you must first compare the `author` with `.eq` before the
`title`.

In this case, the best option is probably to create the separate `by_title`
index to facilitate this query.

## 总结 \{#conclusions\}

恭喜！你现在已经理解了 Convex 中查询和索引是如何工作的！

下面是本节的主要要点：

1. 默认情况下，Convex 查询是&#95;全表扫描&#95;。这适合用于原型开发以及在小表上进行查询。
2. 随着数据表规模增大，你可以通过添加&#95;索引&#95;来提升查询性能。索引是独立的数据结构，会对文档进行排序，以支持快速查询。
3. 在 Convex 中，查询使用 *`withIndex`* 方法来描述查询中使用索引的那一部分。查询的性能取决于索引范围表达式中包含的文档数量。
4. Convex 还支持&#95;复合索引&#95;，可以同时对多个字段建立索引。

要进一步了解查询和索引，请查看以下参考文档：

* [读取数据](/database/reading-data/reading-data.mdx)
* [索引](/database/reading-data/indexes/indexes.md)