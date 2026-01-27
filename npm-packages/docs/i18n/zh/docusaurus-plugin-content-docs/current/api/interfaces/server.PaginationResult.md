---
id: "server.PaginationResult"
title: "接口：PaginationResult<T>"
custom_edit_url: null
---

[server](../modules/server.md).PaginationResult

使用 [paginate](server.OrderedQuery.md#paginate) 进行分页后的结果。

## 类型参数 \{#type-parameters\}

| 名称 |
| :------ |
| `T` |

## 属性 \{#properties\}

### page \{#page\}

• **page**: `T`[]

结果页的数据。

#### 定义于 \{#defined-in\}

[server/pagination.ts:32](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L32)

***

### isDone \{#isdone\}

• **isDone**: `boolean`

我们是否已经到达结果集的末尾？

#### 定义于 \{#defined-in\}

[server/pagination.ts:37](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L37)

***

### continueCursor \{#continuecursor\}

• **continueCursor**: `string`

用于继续加载后续结果的[Cursor](../modules/server.md#cursor)。

#### 定义于 \{#defined-in\}

[server/pagination.ts:42](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L42)

***

### splitCursor \{#splitcursor\}

• `Optional` **splitCursor**: `null` | `string`

一个用于将页面拆分为两部分的 [Cursor](../modules/server.md#cursor)，这样
(cursor, continueCursor] 这一页可以被替换为两页：(cursor, splitCursor]
和 (splitCursor, continueCursor]。

#### 定义于 \{#defined-in\}

[server/pagination.ts:49](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L49)

***

### pageStatus \{#pagestatus\}

• `Optional` **pageStatus**: `null` | `"SplitRecommended"` | `"SplitRequired"`

当某个查询读取的数据过多时，它可能返回 `'SplitRecommended'`，
表示应该使用 `splitCursor` 将当前结果页拆分成两页。
当某个查询读取的数据多到 `page` 可能不完整时，它的状态
会变为 `'SplitRequired'`。

#### 定义于 \{#defined-in\}

[server/pagination.ts:57](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L57)