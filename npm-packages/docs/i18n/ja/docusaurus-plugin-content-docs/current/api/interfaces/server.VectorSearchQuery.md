---
id: "server.VectorSearchQuery"
title: "インターフェース: VectorSearchQuery<TableInfo, IndexName>"
custom_edit_url: null
---

[server](../modules/server.md).VectorSearchQuery

ベクターインデックスに対するベクター検索のためのパラメータを指定するオブジェクトです。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableInfo` | [`GenericTableInfo`](../modules/server.md#generictableinfo) を継承 |
| `IndexName` | [`VectorIndexNames`](../modules/server.md#vectorindexnames)&lt;`TableInfo`&gt; を継承 |

## プロパティ \{#properties\}

### vector \{#vector\}

• **vector**: `number`[]

クエリに使用するベクトル。

これはインデックスの `dimensions` と同じ長さである必要があります。
このベクトル検索では、このベクトルに最も類似したドキュメントの ID が返されます。

#### 定義元 \{#defined-in\}

[server/vector&#95;search.ts:30](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L30)

***

### limit \{#limit\}

• `Optional` **limit**: `number`

返す結果の件数。指定する場合は、1 以上 256 以下でなければなりません。

**`Default`**

```ts
10
```

#### 定義元 \{#defined-in\}

[server/vector&#95;search.ts:37](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L37)

***

### filter \{#filter\}

• `Optional` **filter**: (`q`: [`VectorFilterBuilder`](server.VectorFilterBuilder.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, [`NamedVectorIndex`](../modules/server.md#namedvectorindex)&lt;`TableInfo`, `IndexName`&gt;&gt;) =&gt; [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

#### 型宣言 \{#type-declaration\}

▸ (`q`): [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

インデックスのフィルター用フィールドに対して動作する、`q.or` と `q.eq` から構成されるオプションのフィルター式。

例: `filter: q => q.or(q.eq("genre", "comedy"), q.eq("genre", "drama"))`

##### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `q` | [`VectorFilterBuilder`](server.VectorFilterBuilder.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, [`NamedVectorIndex`](../modules/server.md#namedvectorindex)&lt;`TableInfo`, `IndexName`&gt;&gt; |

##### 戻り値 \{#returns\}

[`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

#### 定義場所 \{#defined-in\}

[server/vector&#95;search.ts:47](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L47)