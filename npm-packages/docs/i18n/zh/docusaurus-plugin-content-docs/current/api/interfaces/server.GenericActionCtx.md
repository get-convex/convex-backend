---
id: "server.GenericActionCtx"
title: "接口: GenericActionCtx<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericActionCtx

在 Convex 操作函数中使用的一组服务。

该上下文会作为第一个参数传递给在服务器上运行的任何 Convex 操作。

如果你在使用代码生成，请在 `convex/_generated/server.d.ts` 中使用 `ActionCtx` 类型，它会根据你的数据模型提供类型定义。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | 扩展自 [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## 属性 \{#properties\}

### scheduler \{#scheduler\}

• **scheduler**: [`Scheduler`](server.Scheduler.md)

用于安排 Convex 函数在未来执行的工具。

#### 定义于 \{#defined-in\}

[server/registration.ts:236](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L236)

***

### auth \{#auth\}

• **auth**: [`Auth`](server.Auth.md)

当前已通过认证的用户的信息。

#### 定义于 \{#defined-in\}

[server/registration.ts:241](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L241)

***

### storage \{#storage\}

• **storage**: [`StorageActionWriter`](server.StorageActionWriter.md)

用于在存储中读写文件的工具。

#### 定义于 \{#defined-in\}

[server/registration.ts:246](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L246)

## 方法 \{#methods\}

### runQuery \{#runquery\}

▸ **runQuery**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

使用指定名称和参数运行 Convex 查询。

建议使用 internalQuery，以防止用户直接调用该查询。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`, `"public"` | `"internal"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `query` | `Query` | 要执行的查询的 [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; | 传给该查询函数的参数。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

一个解析为该查询结果的 Promise。

#### 定义于 \{#defined-in\}

[server/registration.ts:196](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L196)

***

### runMutation \{#runmutation\}

▸ **runMutation**&lt;`Mutation`&gt;(`mutation`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

根据给定的名称和参数运行一个 Convex 变更函数。

建议使用 `internalMutation` 来防止用户直接调用该变更。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`, `"public"` | `"internal"`&gt; |

#### 参数 \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | 要运行的变更的 [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Mutation`&gt; | 传递给该变更函数的参数。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

一个会解析为该变更结果的 Promise。

#### 定义于 \{#defined-in\}

[server/registration.ts:211](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L211)

***

### runAction \{#runaction\}

▸ **runAction**&lt;`Action`&gt;(`action`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

使用给定的名称和参数运行 Convex 操作。

建议使用 internalAction 来防止用户直接调用该操作。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"action"`, `"public"` | `"internal"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `action` | `Action` | 要运行的该操作对应的 [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Action`&gt; | 传递给该操作函数的参数。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

一个解析为该操作结果的 Promise。

#### 定义于 \{#defined-in\}

[server/registration.ts:228](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L228)

***

### vectorSearch \{#vectorsearch\}

▸ **vectorSearch**&lt;`TableName`, `IndexName`&gt;(`tableName`, `indexName`, `query`): `Promise`&lt;&#123; `_id`: [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; ; `_score`: `number`  &#125;[]&gt;

在指定的表和索引上执行向量搜索。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |
| `IndexName` | extends `string` | `number` | `symbol` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `tableName` | `TableName` | 要查询的表名。 |
| `indexName` | `IndexName` | 要查询的该表上的向量索引名称。 |
| `query` | `Object` | 一个 [VectorSearchQuery](server.VectorSearchQuery.md)，其中包含要查询的向量、要返回的结果数量以及任意过滤条件。 |
| `query.vector` | `number`[] | 查询向量。其长度必须与索引的 `dimensions` 相同。该向量搜索会返回与此向量最相似的文档 ID。 |
| `query.limit?` | `number` | 要返回的结果数量。如果指定，必须在 1 到 256（含）之间。**`默认值`** `ts 10 ` |
| `query.filter?` | (`q`: [`VectorFilterBuilder`](server.VectorFilterBuilder.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;, [`NamedVectorIndex`](../modules/server.md#namedvectorindex)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;, `IndexName`&gt;&gt;) =&gt; [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt; | 可选的过滤表达式，由作用于索引过滤字段的 `q.or` 和 `q.eq` 组合而成。例如：`filter: q => q.or(q.eq("genre", "comedy"), q.eq("genre", "drama"))` |

#### 返回值 \{#returns\}

`Promise`&lt;&#123; `_id`: [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; ; `_score`: `number`  &#125;[]&gt;

一个 Promise，包含与最近向量对应文档的 `_id` 和 `_score`

#### 定义于 \{#defined-in\}

[server/registration.ts:258](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L258)