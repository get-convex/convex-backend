---
id: "server.GenericMutationCtx"
title: "接口：GenericMutationCtx<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericMutationCtx

一组可在 Convex 变更函数中使用的服务。

变更上下文会作为第一个参数传递给在服务器上运行的任何 Convex 变更函数。

如果你在使用代码生成，请在 `convex/_generated/server.d.ts` 中使用 `MutationCtx` 类型，
它会针对你的数据模型进行类型定义。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | 扩展自 [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## 属性 \{#properties\}

### db \{#db\}

• **db**: [`GenericDatabaseWriter`](server.GenericDatabaseWriter.md)&lt;`DataModel`&gt;

用于在数据库中读取和写入数据的工具。

#### 定义在 \{#defined-in\}

[server/registration.ts:50](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L50)

***

### auth \{#auth\}

• **auth**: [`Auth`](server.Auth.md)

当前已通过身份验证的用户的信息。

#### 定义于 \{#defined-in\}

[server/registration.ts:55](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L55)

***

### storage \{#storage\}

• **storage**: [`StorageWriter`](server.StorageWriter.md)

用于读取和写入存储中文件的工具。

#### 定义于 \{#defined-in\}

[server/registration.ts:60](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L60)

***

### scheduler \{#scheduler\}

• **scheduler**: [`Scheduler`](server.Scheduler.md)

用于安排 Convex 函数在未来执行的工具。

#### 定义于 \{#defined-in\}

[server/registration.ts:65](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L65)

***

### runQuery \{#runquery\}

• **runQuery**: &lt;Query&gt;(`query`: `Query`, ...`args`: [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt;) =&gt; `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

#### 类型声明 \{#type-declaration\}

▸ &lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

在同一事务中调用查询函数。

注意：通常你可以直接调用该查询函数，而无需使用此方法。
`runQuery` 会产生额外开销，用于执行参数和返回值校验，
以及创建新的独立 JS 上下文。

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

[server/registration.ts:74](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L74)

***

### runMutation \{#runmutation\}

• **runMutation**: &lt;Mutation&gt;(`mutation`: `Mutation`, ...`args`: [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Mutation`&gt;) =&gt; `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

#### 类型声明 \{#type-declaration\}

▸ &lt;`Mutation`&gt;(`mutation`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

在同一个事务中调用一个变更函数。

注意：通常你可以直接调用该变更函数，而不必使用 `runMutation`。
`runMutation` 会带来额外开销：执行参数和返回值校验，
并创建一个新的、隔离的 JS 上下文环境。

该变更在一个子事务中运行，因此如果变更抛出错误，
它的所有写入都会被回滚。另外，成功的变更所做的写入
会与该事务中的其他写入保持可串行化。

##### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`, `"public"` | `"internal"`&gt; |

##### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `mutation` | `Mutation` |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Mutation`&gt; |

##### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

#### 定义于 \{#defined-in\}

[server/registration.ts:90](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L90)