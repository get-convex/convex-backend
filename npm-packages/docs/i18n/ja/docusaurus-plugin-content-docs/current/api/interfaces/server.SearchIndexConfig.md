---
id: "server.SearchIndexConfig"
title: "インターフェース: SearchIndexConfig<SearchField, FilterFields>"
custom_edit_url: null
---

[server](../modules/server.md).SearchIndexConfig

全文検索インデックスの設定。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `SearchField` | extends `string` |
| `FilterFields` | extends `string` |

## プロパティ \{#properties\}

### searchField \{#searchfield\}

• **searchField**: `SearchField`

全文検索用にインデックス化するフィールドです。

これは `string` 型のフィールドでなければなりません。

#### 定義元 \{#defined-in\}

[server/schema.ts:101](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L101)

***

### filterFields \{#filterfields\}

• `Optional` **filterFields**: `FilterFields`[]

検索クエリの実行時に、高速にフィルタリングできるようインデックス化する追加フィールド。

#### 定義元 \{#defined-in\}

[server/schema.ts:106](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L106)