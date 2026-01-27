---
id: "server.Query"
title: "接口：Query<TableInfo>"
custom_edit_url: null
---

[server](../modules/server.md).Query

[Query](server.Query.md) 接口允许函数从数据库中读取值。

**如果你只需要按 ID 加载一个对象，请改用 `db.get(id)`。**

执行一次查询包括以下步骤：

1. （可选）调用 [order](server.Query.md#order) 定义排序顺序
2. （可选）调用 [filter](server.OrderedQuery.md#filter) 细化结果
3. 调用 *consumer* 方法获取结果

查询是惰性求值的。在开始迭代之前不会做任何工作，因此构造和扩展查询几乎没有任何开销。查询会在结果被迭代时逐步执行，因此提前终止也会降低查询的开销。

使用 `filter` 表达式要比用 JavaScript 来过滤更高效。

|                                              | |
|----------------------------------------------|-|
| **Ordering**                                 | |
| [`order("asc")`](#order)                     | 定义查询结果的排序顺序。 |
|                                              | |
| **Filtering**                                | |
| [`filter(...)`](#filter)                     | 过滤查询结果，仅保留满足某些条件的值。 |
|                                              | |
| **Consuming**                                | 以不同方式执行查询并返回结果。 |
| [`[Symbol.asyncIterator]()`](#asynciterator) | 查询结果可以通过 `for await..of` 循环进行迭代。 |
| [`collect()`](#collect)                      | 将所有结果作为数组返回。 |
| [`take(n: number)`](#take)                   | 将前 `n` 个结果作为数组返回。 |
| [`first()`](#first)                          | 返回第一个结果。 |
| [`unique()`](#unique)                        | 返回唯一的结果，如果结果多于一个则抛出异常。 |

要进一步了解如何编写查询，请参阅 [Querying the Database](https://docs.convex.dev/using/database-queries)。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableInfo` | 扩展自 [`GenericTableInfo`](../modules/server.md#generictableinfo) |

## 继承层次结构 \{#hierarchy\}

* [`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

  ↳ **`Query`**

  ↳↳ [`QueryInitializer`](server.QueryInitializer.md)

## 方法 \{#methods\}

### [asyncIterator]

▸ **[asyncIterator]**(): `AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### 返回值 \{#returns\}

`AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### 继承自 \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[[asyncIterator]](server.OrderedQuery.md#[asynciterator])

#### 定义在 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es2018.asynciterable.d.ts:38

***

### order \{#order\}

▸ **order**(`order`): [`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

定义查询结果的排序方式。

使用 `"asc"` 表示升序，使用 `"desc"` 表示降序。如果未指定，默认使用升序。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `order` | `"asc"` | `"desc"` | 指定返回结果的顺序。 |

#### 返回值 \{#returns\}

[`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

#### 定义于 \{#defined-in\}

[server/query.ts:149](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L149)

***

### filter \{#filter\}

▸ **filter**(`predicate`): [`Query`](server.Query.md)&lt;`TableInfo`&gt;

过滤查询输出，仅返回使 `predicate` 的计算结果为 true 的值。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `predicate` | (`q`: [`FilterBuilder`](server.FilterBuilder.md)&lt;`TableInfo`&gt;) =&gt; [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt; | 使用提供的 [FilterBuilder](server.FilterBuilder.md) 构造的 [Expression](../classes/server.Expression.md)，用于指定应保留哪些文档。 |

#### 返回值 \{#returns\}

[`Query`](server.Query.md)&lt;`TableInfo`&gt;

* 带有指定过滤谓词的新 [OrderedQuery](server.OrderedQuery.md)。

#### 继承自 \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[filter](server.OrderedQuery.md#filter)

#### 定义于 \{#defined-in\}

[server/query.ts:165](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L165)

***

### paginate \{#paginate\}

▸ **paginate**(`paginationOpts`): `Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

加载一页包含 `n` 个结果的数据，并获取一个用于加载更多数据的 [Cursor](../modules/server.md#cursor)。

注意：如果在响应式查询函数中调用此方法，返回结果的数量可能与
`paginationOpts.numItems` 不匹配！

`paginationOpts.numItems` 只是一个初始值。第一次调用之后，
`paginate` 将返回原始查询范围内的所有项。这样可以确保所有页面保持相邻且互不重叠。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `paginationOpts` | [`PaginationOptions`](server.PaginationOptions.md) | 一个 [PaginationOptions](server.PaginationOptions.md) 对象，包含要加载的条目数量以及起始游标。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

一个 [PaginationResult](server.PaginationResult.md)，其中包含一页查询结果以及一个用于继续分页的游标。

#### 继承自 \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[paginate](server.OrderedQuery.md#paginate)

#### 定义于 \{#defined-in\}

[server/query.ts:194](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L194)

***

### collect \{#collect\}

▸ **collect**(): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

执行查询，并以数组形式返回全部结果。

注意：在处理返回结果很多的查询时，通常更好的做法是将该 `Query` 作为
`AsyncIterable` 来迭代使用。

#### 返回值 \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* 由查询的所有结果组成的数组。

#### 继承自 \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[collect](server.OrderedQuery.md#collect)

#### 定义于 \{#defined-in\}

[server/query.ts:206](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L206)

***

### take \{#take\}

▸ **take**(`n`): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

执行查询并返回前 `n` 条结果。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `n` | `number` | 要获取的元素数量。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* 包含查询前 `n` 个结果的数组（如果查询结果少于 `n` 个，则为对应数量的结果）。

#### 继承自 \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[take](server.OrderedQuery.md#take)

#### 定义于 \{#defined-in\}

[server/query.ts:215](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L215)

***

### first \{#first\}

▸ **first**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

执行查询，如果有结果则返回第一个结果。

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* 查询的第一个结果；如果查询未返回任何结果，则为 `null`。

#### 继承自 \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[first](server.OrderedQuery.md#first)

#### 定义于 \{#defined-in\}

[server/query.ts:222](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L222)

***

### unique \{#unique\}

▸ **unique**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

执行该查询，并在有结果时返回单个结果。

**`Throws`**

当查询返回多个结果时，将抛出错误。

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* 查询返回的唯一结果；若不存在则为 null。

#### 继承自 \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[unique](server.OrderedQuery.md#unique)

#### 定义于 \{#defined-in\}

[server/query.ts:230](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L230)