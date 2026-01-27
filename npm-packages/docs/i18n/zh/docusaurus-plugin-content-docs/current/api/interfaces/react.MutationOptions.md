---
id: "react.MutationOptions"
title: "接口：MutationOptions<Args>"
custom_edit_url: null
---

[react](../modules/react.md).MutationOptions

用于[变更](../classes/react.ConvexReactClient.md#mutation)的选项。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Args` | extends `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; |

## 属性 \{#properties\}

### optimisticUpdate \{#optimisticupdate\}

• `Optional` **optimisticUpdate**: [`OptimisticUpdate`](../modules/browser.md#optimisticupdate)&lt;`Args`&gt;

一个会与此变更一同应用的乐观更新。

乐观更新会在某个变更处于挂起状态时，本地更新查询。
一旦该变更完成，此更新将被回滚。

#### 定义于 \{#defined-in\}

[react/client.ts:282](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L282)