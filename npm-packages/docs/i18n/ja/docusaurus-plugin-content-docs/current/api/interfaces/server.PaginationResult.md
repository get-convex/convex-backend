---
id: "server.PaginationResult"
title: "インターフェイス: PaginationResult<T>"
custom_edit_url: null
---

[server](../modules/server.md).PaginationResult

[paginate](server.OrderedQuery.md#paginate) を使用してページネーションした結果。

## 型パラメーター \{#type-parameters\}

| 名前 |
| :------ |
| `T` |

## プロパティ \{#properties\}

### page \{#page\}

• **page**: `T`[]

このページに含まれる結果。

#### 定義元 \{#defined-in\}

[server/pagination.ts:32](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L32)

***

### isDone \{#isdone\}

• **isDone**: `boolean`

結果をすべて取得し終えましたか？

#### 定義元 \{#defined-in\}

[server/pagination.ts:37](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L37)

***

### continueCursor \{#continuecursor\}

• **continueCursor**: `string`

結果の読み込みを継続するための[Cursor](../modules/server.md#cursor)。

#### 定義場所 \{#defined-in\}

[server/pagination.ts:42](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L42)

***

### splitCursor \{#splitcursor\}

• `Optional` **splitCursor**: `null` | `string`

ページを 2 つに分割するための [Cursor](../modules/server.md#cursor)。\
(cursor, continueCursor] の範囲のページを、(cursor, splitCursor] と
(splitCursor, continueCursor] の 2 つのページに分割できます。

#### 定義場所 \{#defined-in\}

[server/pagination.ts:49](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L49)

***

### pageStatus \{#pagestatus\}

• `Optional` **pageStatus**: `null` | `"SplitRecommended"` | `"SplitRequired"`

クエリが読み込むデータ量が多すぎる場合、`splitCursor` を使ってページを 2 つに分割すべきであることを示すために、&#39;SplitRecommended&#39; が返されることがあります。
クエリが非常に多くのデータを読み込んで `page` が不完全になる可能性がある場合、その status は &#39;SplitRequired&#39; になります。

#### 定義場所 \{#defined-in\}

[server/pagination.ts:57](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L57)