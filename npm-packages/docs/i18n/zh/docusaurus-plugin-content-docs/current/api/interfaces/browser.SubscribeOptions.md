---
id: "browser.SubscribeOptions"
title: "接口：SubscribeOptions"
custom_edit_url: null
---

[browser](../modules/browser.md).SubscribeOptions

用于 [subscribe](../classes/browser.BaseConvexClient.md#subscribe) 的选项。

## 属性 \{#properties\}

### journal \{#journal\}

• `Optional` **journal**: [`QueryJournal`](../modules/browser.md#queryjournal)

由之前执行此查询函数生成的（可选）journal。

如果已经存在针对名称和参数相同的查询函数的订阅，则该 journal 不会产生任何效果。

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:190](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L190)