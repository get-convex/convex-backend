---
id: "server.PaginationOptions"
title: "接口：PaginationOptions"
custom_edit_url: null
---

[server](../modules/server.md).PaginationOptions

传递给 [paginate](server.OrderedQuery.md#paginate) 的选项。

要在 [参数验证](https://docs.convex.dev/functions/validation) 中使用此类型，
请使用 [paginationOptsValidator](../modules/server.md#paginationoptsvalidator)。

## 属性 \{#properties\}

### numItems \{#numitems\}

• **numItems**: `number`

在此结果页中要加载的条目数量。

注意：这只是一个初始值！

如果你在响应式查询函数中运行这个分页查询，当有条目被添加到或从查询范围中移除时，你实际收到的条目数量可能会多于或少于该值。

#### 定义于 \{#defined-in\}

[server/pagination.ts:78](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L78)

***

### cursor \{#cursor\}

• **cursor**: `null` | `string`

一个表示当前页起始位置的 [Cursor](../modules/server.md#cursor)，或者为 `null`，以从查询结果的开头开始。

#### 定义于 \{#defined-in\}

[server/pagination.ts:84](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L84)