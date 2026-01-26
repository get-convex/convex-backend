---
id: "server.GenericQueryCtx"
title: "接口：GenericQueryCtx<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericQueryCtx

在 Convex 查询函数中使用的一组服务。

查询上下文会作为第一个参数传递给在服务端运行的任何 Convex 查询函数。

这与 MutationCtx 不同，因为其中提供的所有服务都是只读的。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | 继承自 [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## 属性 \{#properties\}

### db \{#db\}

• **db**: [`GenericDatabaseReader`](server.GenericDatabaseReader.md)&lt;`DataModel`&gt;

用于读取数据库数据的工具。

#### 定义于 \{#defined-in\}

[server/registration.ts:130](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L130)

***

### auth \{#auth\}

• **auth**: [`Auth`](server.Auth.md)

当前已通过身份验证的用户信息。

#### 定义于 \{#defined-in\}

[server/registration.ts:135](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L135)

***

### storage \{#storage\}

• **storage**: [`StorageReader`](server.StorageReader.md)

用于读取存储中的文件的工具。

#### 定义于 \{#defined-in\}

[server/registration.ts:140](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L140)

***

### runQuery \{#runquery\}

• **runQuery**: &lt;Query&gt;(`query`: `Query`, ...`args`: [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt;) =&gt; `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

#### 类型声明 \{#type-declaration\}

▸ &lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

在同一事务中调用一个查询函数。

注意：很多情况下，你可以直接调用该查询函数，而不必使用它。
`runQuery` 会带来参数和返回值校验，
以及创建新的隔离 JS 上下文的开销。

##### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`, `"public"` | `"internal"`&gt; |

##### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `query` | `Query` |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; |

##### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

#### 定义于 \{#defined-in\}

[server/registration.ts:149](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L149)