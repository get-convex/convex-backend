---
id: "react.WatchQueryOptions"
title: "接口：WatchQueryOptions"
custom_edit_url: null
---

[react](../modules/react.md).WatchQueryOptions

用于 [watchQuery](../classes/react.ConvexReactClient.md#watchquery) 的选项。

## 属性 \{#properties\}

### journal \{#journal\}

• `Optional` **journal**: [`QueryJournal`](../modules/browser.md#queryjournal)

由此前执行该查询函数生成的（可选）日志。

如果已经存在对同名且参数相同的查询函数的订阅，则此 journal 不会产生任何效果。

#### 定义于 \{#defined-in\}

[react/client.ts:241](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L241)