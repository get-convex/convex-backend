---
id: "server.VectorSearchQuery"
title: "接口：VectorSearchQuery<TableInfo, IndexName>"
custom_edit_url: null
---

[server](../modules/server.md).VectorSearchQuery

一个带参数的对象，用于在向量索引上执行向量搜索。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableInfo` | 扩展自 [`GenericTableInfo`](../modules/server.md#generictableinfo) |
| `IndexName` | 扩展自 [`VectorIndexNames`](../modules/server.md#vectorindexnames)&lt;`TableInfo`&gt; |

## 属性 \{#properties\}

### vector \{#vector\}

• **vector**: `number`[]

查询向量。

该向量的长度必须与索引的 `dimensions` 相同。
此向量搜索将返回与该向量最相似的文档的 ID。

#### 定义于 \{#defined-in\}

[server/vector&#95;search.ts:30](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L30)

***

### limit \{#limit\}

• `可选` **limit**: `number`

要返回的结果数量。如果指定，必须在 1 到 256（含）之间。

**`默认`**

```ts
10
```

#### 定义于 \{#defined-in\}

[server/vector&#95;search.ts:37](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L37)

***

### filter \{#filter\}

• `Optional` **filter**: (`q`: [`VectorFilterBuilder`](server.VectorFilterBuilder.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, [`NamedVectorIndex`](../modules/server.md#namedvectorindex)&lt;`TableInfo`, `IndexName`&gt;&gt;) =&gt; [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

#### 类型声明 \{#type-declaration\}

▸ (`q`): [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

可选的过滤表达式，由 `q.or` 和 `q.eq` 组合而成，用于该索引的过滤字段。

例如：`filter: q => q.or(q.eq("genre", "comedy"), q.eq("genre", "drama"))`

##### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `q` | [`VectorFilterBuilder`](server.VectorFilterBuilder.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, [`NamedVectorIndex`](../modules/server.md#namedvectorindex)&lt;`TableInfo`, `IndexName`&gt;&gt; |

##### 返回 \{#returns\}

[`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

#### 定义于 \{#defined-in\}

[server/vector&#95;search.ts:47](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L47)