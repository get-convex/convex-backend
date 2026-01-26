---
id: "server.QueryInitializer"
title: "接口：QueryInitializer<TableInfo>"
custom_edit_url: null
---

[server](../modules/server.md).QueryInitializer

[QueryInitializer](server.QueryInitializer.md) 接口是针对 Convex 数据库表构建[查询](server.Query.md)的入口点。

查询有两种类型：

1. 全表扫描：使用 [fullTableScan](server.QueryInitializer.md#fulltablescan) 创建的查询，
   按插入顺序遍历表中的所有文档。
2. 索引查询：使用 [withIndex](server.QueryInitializer.md#withindex) 创建的查询，
   按索引顺序遍历索引范围。

为方便使用，[QueryInitializer](server.QueryInitializer.md) 继承了 [Query](server.Query.md) 接口，并会隐式地开始一次全表扫描。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableInfo` | 继承自 [`GenericTableInfo`](../modules/server.md#generictableinfo) |

## 层次结构 \{#hierarchy\}

* [`Query`](server.Query.md)&lt;`TableInfo`&gt;

  ↳ **`QueryInitializer`**

## 方法 \{#methods\}

### [asyncIterator]

▸ **[asyncIterator]**(): `AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### 返回值 \{#returns\}

`AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### 继承自 \{#inherited-from\}

[Query](server.Query.md).[[asyncIterator]](server.Query.md#[asynciterator])

#### 定义于 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es2018.asynciterable.d.ts:38

***

### fullTableScan \{#fulltablescan\}

▸ **fullTableScan**(): [`Query`](server.Query.md)&lt;`TableInfo`&gt;

通过读取该表中的所有值来执行查询。

此查询的开销与整张表的大小成正比，因此只应在会保持非常小（例如几百到几千条文档）且更新不频繁的表上使用。

#### 返回值 \{#returns\}

[`Query`](server.Query.md)&lt;`TableInfo`&gt;

* 用于遍历该表中所有文档的 [查询](server.Query.md)。

#### 定义于 \{#defined-in\}

[server/query.ts:40](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L40)

***

### withIndex \{#withindex\}

▸ **withIndex**&lt;`IndexName`&gt;(`indexName`, `indexRange?`): [`Query`](server.Query.md)&lt;`TableInfo`&gt;

通过从此表的某个索引中读取文档来进行查询。

此查询的开销与匹配索引范围表达式的文档数量成正比。

结果将按照索引顺序返回。

要了解索引相关内容，请参阅 [Indexes](https://docs.convex.dev/using/indexes)。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `IndexName` | extends `string` | `number` | `symbol` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `indexName` | `IndexName` | 要查询的索引名称。 |
| `indexRange?` | (`q`: [`IndexRangeBuilder`](server.IndexRangeBuilder.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, [`NamedIndex`](../modules/server.md#namedindex)&lt;`TableInfo`, `IndexName`&gt;, `0`&gt;) =&gt; [`IndexRange`](../classes/server.IndexRange.md) | 使用提供的 [IndexRangeBuilder](server.IndexRangeBuilder.md) 构造的可选索引范围。索引范围用于描述 Convex 在执行查询时应考虑哪些文档。如果未指定索引范围，则查询会考虑该索引中的所有文档。 |

#### 返回值 \{#returns\}

[`Query`](server.Query.md)&lt;`TableInfo`&gt;

* 返回该索引中文档的查询。

#### 定义于 \{#defined-in\}

[server/query.ts:59](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L59)

***

### withSearchIndex \{#withsearchindex\}

▸ **withSearchIndex**&lt;`IndexName`&gt;(`indexName`, `searchFilter`): [`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

通过在搜索索引上执行全文搜索来进行查询。

搜索查询必须始终在索引的 `searchField` 字段中搜索文本。该查询还可以为索引中指定的任意 `filterFields` 可选地添加等值过滤条件。

返回的文档将根据与搜索文本匹配程度的相关性排序。

要了解全文搜索的更多信息，请参阅 [Indexes](https://docs.convex.dev/text-search)。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `IndexName` | extends `string` | `number` | `symbol` |

#### 参数 \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `indexName` | `IndexName` | 要查询的搜索索引名称。 |
| `searchFilter` | (`q`: [`SearchFilterBuilder`](server.SearchFilterBuilder.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, [`NamedSearchIndex`](../modules/server.md#namedsearchindex)&lt;`TableInfo`, `IndexName`&gt;&gt;) =&gt; [`SearchFilter`](../classes/server.SearchFilter.md) | 使用提供的 [SearchFilterBuilder](server.SearchFilterBuilder.md) 构造的搜索过滤表达式。它定义了要执行的全文搜索，以及在该搜索索引内要执行的等值过滤条件。 |

#### 返回值 \{#returns\}

[`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

* 一个查询，用于搜索匹配的文档，并按相关性排序返回它们。

#### 定义于 \{#defined-in\}

[server/query.ts:88](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L88)

***

### order \{#order\}

▸ **order**(`order`): [`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

定义查询结果的排序方式。

使用 `"asc"` 表示升序，使用 `"desc"` 表示降序。如果未指定，则默认为升序排序。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `order` | `"asc"` | `"desc"` | 返回结果的顺序。 |

#### 返回值 \{#returns\}

[`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

#### 继承自 \{#inherited-from\}

[Query](server.Query.md).[order](server.Query.md#order)

#### 定义于 \{#defined-in\}

[server/query.ts:149](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L149)

***

### filter \{#filter\}

▸ **filter**(`predicate`): [`QueryInitializer`](server.QueryInitializer.md)&lt;`TableInfo`&gt;

过滤查询输出，仅返回使 `predicate` 返回 true 的值。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `predicate` | (`q`: [`FilterBuilder`](server.FilterBuilder.md)&lt;`TableInfo`&gt;) =&gt; [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt; | 使用提供的 [FilterBuilder](server.FilterBuilder.md) 构建的 [Expression](../classes/server.Expression.md)，用于指定要保留哪些文档。 |

#### 返回值 \{#returns\}

[`QueryInitializer`](server.QueryInitializer.md)&lt;`TableInfo`&gt;

* 一个应用了给定过滤谓词的新 [OrderedQuery](server.OrderedQuery.md)。

#### 继承自 \{#inherited-from\}

[Query](server.Query.md).[filter](server.Query.md#filter)

#### 定义于 \{#defined-in\}

[server/query.ts:165](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L165)

***

### paginate \{#paginate\}

▸ **paginate**(`paginationOpts`): `Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

加载一页包含 `n` 个结果的数据，并获取一个用于加载更多内容的 [Cursor](../modules/server.md#cursor) 游标。

注意：如果在响应式查询函数中调用此方法，返回结果的数量可能与
`paginationOpts.numItems` 不一致！

`paginationOpts.numItems` 只是一个初始值。在第一次调用之后，
`paginate` 将返回原始查询范围内的所有项。这样可以确保所有页面彼此相邻且不重叠。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `paginationOpts` | [`PaginationOptions`](server.PaginationOptions.md) | 一个 [PaginationOptions](server.PaginationOptions.md) 对象，包含要加载的条目数量以及起始游标。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

一个 [PaginationResult](server.PaginationResult.md)，其中包含当前页的结果，以及一个用于继续分页的游标。

#### 继承自 \{#inherited-from\}

[Query](server.Query.md).[paginate](server.Query.md#paginate)

#### 定义于 \{#defined-in\}

[server/query.ts:194](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L194)

***

### collect \{#collect\}

▸ **collect**(): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

执行此查询，并将所有结果作为数组返回。

注意：在处理结果数量较多的查询时，通常更推荐将 `Query` 作为
`AsyncIterable` 来使用。

#### 返回值 \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* 包含该查询所有结果的数组。

#### 继承自 \{#inherited-from\}

[Query](server.Query.md).[collect](server.Query.md#collect)

#### 定义于 \{#defined-in\}

[server/query.ts:206](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L206)

***

### take \{#take\}

▸ **take**(`n`): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

执行该查询并返回前 `n` 个结果。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `n` | `number` | 要获取的元素数量。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* 由查询的前 `n` 个结果组成的数组（如果该查询不足 `n` 个结果，则返回实际可用的结果）。

#### 继承自 \{#inherited-from\}

[Query](server.Query.md).[take](server.Query.md#take)

#### 定义于 \{#defined-in\}

[server/query.ts:215](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L215)

***

### first \{#first\}

▸ **first**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

执行查询，并在有结果时返回第一条结果。

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* 查询结果中的第一个值，如果查询没有返回任何结果，则返回 `null`。

#### 继承自 \{#inherited-from\}

[Query](server.Query.md).[first](server.Query.md#first)

#### 定义于 \{#defined-in\}

[server/query.ts:222](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L222)

***

### unique \{#unique\}

▸ **unique**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

执行查询，如果存在单个结果，则返回该结果。

**`Throws`**

如果查询返回多个结果，将抛出错误。

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* 查询返回的单个结果，如果不存在结果则为 null。

#### 继承自 \{#inherited-from\}

[Query](server.Query.md).[unique](server.Query.md#unique)

#### 定义于 \{#defined-in\}

[server/query.ts:230](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L230)