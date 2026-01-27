---
id: "react.ReactMutation"
title: "接口：ReactMutation<Mutation>"
custom_edit_url: null
---

[react](../modules/react.md).ReactMutation

用于在服务器上执行 Convex 变更函数的接口。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`&gt; |

## 可调用 \{#callable\}

### ReactMutation \{#reactmutation\}

▸ **ReactMutation**(`...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

在服务器上执行该变更，并返回一个包含其返回值的 `Promise`。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Mutation`&gt; | 用于传递到服务器的变更参数。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

服务端函数调用的返回值。

#### 定义于 \{#defined-in\}

[react/client.ts:64](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L64)

## 方法 \{#methods\}

### withOptimisticUpdate \{#withoptimisticupdate\}

▸ **withOptimisticUpdate**&lt;`T`&gt;(`optimisticUpdate`): [`ReactMutation`](react.ReactMutation.md)&lt;`Mutation`&gt;

定义一个作为此变更一部分应用的乐观更新。

这是对本地查询结果的临时更新，用于支持快速、可交互的 UI。它允许在服务器上的变更执行之前就更新查询结果。

当变更被调用时，将应用这次乐观更新。

乐观更新也可以用于暂时从客户端移除查询，并在变更完成且新的查询结果同步之前，营造加载中的体验。

当变更完全完成且查询已更新后，此更新将自动回滚。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | extends [`OptimisticUpdate`](../modules/browser.md#optimisticupdate)&lt;[`FunctionArgs`](../modules/server.md#functionargs)&lt;`Mutation`&gt;&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `optimisticUpdate` | `T` &amp; `ReturnType`&lt;`T`&gt; extends `Promise`&lt;`any`&gt; ? `"Optimistic update handlers must be synchronous"` : {} | 要应用的乐观更新逻辑。 |

#### 返回值 \{#returns\}

[`ReactMutation`](react.ReactMutation.md)&lt;`Mutation`&gt;

带有已配置更新的新 `ReactMutation` 实例。

#### 定义于 \{#defined-in\}

[react/client.ts:87](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L87)