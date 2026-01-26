---
id: "browser.MutationOptions"
title: "接口：MutationOptions"
custom_edit_url: null
---

[browser](../modules/browser.md).MutationOptions

用于执行 [mutation](../classes/browser.BaseConvexClient.md#mutation) 时的选项。

## 属性 \{#properties\}

### optimisticUpdate \{#optimisticupdate\}

• `Optional` **optimisticUpdate**: [`OptimisticUpdate`](../modules/browser.md#optimisticupdate)&lt;`any`&gt;

与此变更一起应用的乐观更新。

乐观更新会在变更处于挂起状态时本地更新查询。
一旦变更完成，该更新将被回滚。

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:210](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L210)