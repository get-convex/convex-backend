---
id: "server.PaginationOptions"
title: "インターフェース: PaginationOptions"
custom_edit_url: null
---

[server](../modules/server.md).PaginationOptions

[paginate](server.OrderedQuery.md#paginate) に渡すオプションです。

この型を[引数バリデーション](https://docs.convex.dev/functions/validation)で使用するには、
[paginationOptsValidator](../modules/server.md#paginationoptsvalidator) を利用します。

## プロパティ \{#properties\}

### numItems \{#numitems\}

• **numItems**: `number`

この結果ページで読み込むアイテム数。

注意: これはあくまで初期値です！

このページングされたクエリをリアクティブなクエリ関数で実行している場合、
クエリ範囲内でアイテムが追加または削除されると、この値より多い、あるいは少ない
アイテム数が返されることがあります。

#### 定義元 \{#defined-in\}

[server/pagination.ts:78](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L78)

***

### cursor \{#cursor\}

• **cursor**: `null` | `string`

このページの開始位置を表す [Cursor](../modules/server.md#cursor)、またはクエリ結果の先頭から開始する場合は `null` を指定します。

#### 定義場所 \{#defined-in\}

[server/pagination.ts:84](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L84)