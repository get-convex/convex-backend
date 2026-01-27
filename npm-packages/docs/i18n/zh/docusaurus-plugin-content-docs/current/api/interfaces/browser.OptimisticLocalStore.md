---
id: "browser.OptimisticLocalStore"
title: "接口：OptimisticLocalStore"
custom_edit_url: null
---

[browser](../modules/browser.md).OptimisticLocalStore

当前在 Convex 客户端中的查询结果视图，用于执行乐观更新。

## 方法 \{#methods\}

### getQuery \{#getquery\}

▸ **getQuery**&lt;`Query`&gt;(`query`, `...args`): `undefined` | [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;

从客户端获取某个查询的结果。

重要：应将查询结果视为不可变！
始终为查询结果中的数据结构创建新的副本，以避免损坏客户端中的数据。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `query` | `Query` | 要获取的查询对应的 [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; | 此查询的参数对象。 |

#### 返回值 \{#returns\}

`undefined` | [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;

查询结果；如果该查询当前不在客户端中，则返回 `undefined`。

#### 定义于 \{#defined-in\}

[browser/sync/optimistic&#95;updates.ts:28](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/optimistic_updates.ts#L28)

***

### getAllQueries \{#getallqueries\}

▸ **getAllQueries**&lt;`Query`&gt;(`query`): &#123; `args`: [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Query`&gt; ; `value`: `undefined` | [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;  &#125;[]

获取具有给定名称的所有查询的结果及其参数。

这对于需要检查并更新多个查询结果的复杂乐观更新非常有用（例如更新分页列表）。

重要：查询结果应视为不可变！
在查询结果中操作结构时，始终创建新的副本，以避免损坏客户端中的数据。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `query` | `Query` | 要获取的查询对应的 [FunctionReference](../modules/server.md#functionreference)。 |

#### 返回值 \{#returns\}

&#123; `args`: [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Query`&gt; ; `value`: `undefined` | [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;  &#125;[]

一个对象数组，其中每个元素对应一个具有指定名称的查询。
每个对象包含：

* `args` - 此查询的参数对象。
  * `value` 查询结果；如果查询仍在加载中，则为 `undefined`。

#### 定义于 \{#defined-in\}

[browser/sync/optimistic&#95;updates.ts:49](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/optimistic_updates.ts#L49)

***

### setQuery \{#setquery\}

▸ **setQuery**&lt;`Query`&gt;(`query`, `args`, `value`): `void`

乐观地更新某个查询的结果。

可以传入一个新的值（也许是基于旧值计算出来的，旧值可通过
[getQuery](browser.OptimisticLocalStore.md#getquery) 获取），也可以传入 `undefined` 来移除这个查询。
在 Convex 重新计算查询结果期间，移除查询对于创建加载状态非常有用。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `query` | `Query` | 要设置的查询对应的 [FunctionReference](../modules/server.md#functionreference)。 |
| `args` | [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Query`&gt; | 此查询的参数对象。 |
| `value` | `undefined` | [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt; | 要将该查询设置为的新值；传入 `undefined` 可将其从客户端移除。 |

#### 返回 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[browser/sync/optimistic&#95;updates.ts:69](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/optimistic_updates.ts#L69)