---
id: "server.OrderedQuery"
title: "接口：OrderedQuery<TableInfo>"
custom_edit_url: null
---

[server](../modules/server.md).OrderedQuery

具有已定义排序的[查询](server.Query.md)。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableInfo` | 扩展自 [`GenericTableInfo`](../modules/server.md#generictableinfo) |

## 层次结构 \{#hierarchy\}

* `AsyncIterable`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

  ↳ **`OrderedQuery`**

  ↳↳ [`Query`](server.Query.md)

## 方法 \{#methods\}

### [asyncIterator]

▸ **[asyncIterator]**(): `AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### 返回值 \{#returns\}

`AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### 继承自 \{#inherited-from\}

AsyncIterable.[asyncIterator]

#### 定义于 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es2018.asynciterable.d.ts:38

***

### filter \{#filter\}

▸ **filter**(`predicate`): [`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

过滤查询输出，只返回使 `predicate` 求值为 true 的值。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `predicate` | (`q`: [`FilterBuilder`](server.FilterBuilder.md)&lt;`TableInfo`&gt;) =&gt; [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt; | 使用提供的 [FilterBuilder](server.FilterBuilder.md) 构造的 [Expression](../classes/server.Expression.md)，用来指定要保留哪些文档。 |

#### 返回值 \{#returns\}

[`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

* 应用了给定过滤谓词的新的 [OrderedQuery](server.OrderedQuery.md)。

#### 定义于 \{#defined-in\}

[server/query.ts:165](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L165)

***

### paginate \{#paginate\}

▸ **paginate**(`paginationOpts`): `Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

加载一页包含 `n` 条结果的数据，并获取一个用于加载更多数据的 [Cursor](../modules/server.md#cursor)。

注意：如果在响应式查询函数中调用此方法，返回的结果数量可能与
`paginationOpts.numItems` 不一致！

`paginationOpts.numItems` 只是一个初始值。在第一次调用之后，
`paginate` 会返回原始查询范围内的所有项。这样可以确保各页之间保持相邻且不重叠。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `paginationOpts` | [`PaginationOptions`](server.PaginationOptions.md) | 一个 [PaginationOptions](server.PaginationOptions.md) 对象，包含要加载的项数以及起始游标。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

一个 [PaginationResult](server.PaginationResult.md)，其中包含当前结果页以及用于继续分页的游标。

#### 定义于 \{#defined-in\}

[server/query.ts:194](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L194)

***

### collect \{#collect\}

▸ **collect**(): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

执行查询并以数组形式返回全部结果。

注意：在处理结果数量很多的查询时，通常更好的做法是将 `Query` 作为
`AsyncIterable` 使用。

#### 返回值 \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* 查询的所有结果组成的数组。

#### 定义于 \{#defined-in\}

[server/query.ts:206](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L206)

***

### take \{#take\}

▸ **take**(`n`): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

执行查询并返回前 `n` 个结果。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `n` | `number` | 要获取的元素数量。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* 查询结果中前 `n` 个元素组成的数组（如果查询结果少于 `n` 个，则返回包含实际结果数的数组）。

#### 定义于 \{#defined-in\}

[server/query.ts:215](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L215)

***

### first \{#first\}

▸ **first**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

执行查询，如果有结果则返回第一条。

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* 查询的第一个结果；如果查询没有返回结果，则为 `null`。

#### 定义于 \{#defined-in\}

[server/query.ts:222](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L222)

***

### unique \{#unique\}

▸ **unique**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

执行查询，并在存在结果时返回唯一的一条结果。

**`Throws`**

如果查询返回多于一个结果，将抛出错误。

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* 查询返回的单个结果，如果不存在则为 null。

#### 定义于 \{#defined-in\}

[server/query.ts:230](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L230)