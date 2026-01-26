---
id: "server.SearchFilterBuilder"
title: "インターフェース: SearchFilterBuilder<Document, SearchIndexConfig>"
custom_edit_url: null
---

[server](../modules/server.md).SearchFilterBuilder

検索フィルターを定義するためのビルダーです。

検索フィルターは次のようなメソッドチェーンから構成されます:

1. `.search` で構築される 1 つの検索式。
2. `.eq` で構築される 0 個以上の等値式。

検索式は必ずインデックスの `searchField` のテキストを検索しなければなりません。
フィルター式では、インデックスで定義されている任意の `filterFields` を使用できます。

それ以外のフィルタリングには [filter](server.OrderedQuery.md#filter) を使用してください。

全文検索については [Indexes](https://docs.convex.dev/text-search) を参照してください。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Document` | extends [`GenericDocument`](../modules/server.md#genericdocument) |
| `SearchIndexConfig` | extends [`GenericSearchIndexConfig`](../modules/server.md#genericsearchindexconfig) |

## メソッド \{#methods\}

### search \{#search\}

▸ **search**(`fieldName`, `query`): [`SearchFilterFinalizer`](server.SearchFilterFinalizer.md)&lt;`Document`, `SearchIndexConfig`&gt;

`doc[fieldName]` 内で、`query` に含まれる語句を検索します。

これは全文検索を行い、`query` に含まれる単語のいずれかが
そのフィールド内に現れるドキュメントを返します。

ドキュメントはクエリとの関連度に基づいて返されます。これは次の点が考慮されます:

* クエリ内の単語のうち、いくつがテキスト内に現れるか
* それらが何回現れるか
* テキストフィールドの長さ

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `fieldName` | `SearchIndexConfig`[`"searchField"`] | 検索対象とするフィールド名。インデックスの `searchField` に指定されている必要があります。 |
| `query` | `string` | 検索するクエリ文字列。 |

#### 戻り値 \{#returns\}

[`SearchFilterFinalizer`](server.SearchFilterFinalizer.md)&lt;`Document`, `SearchIndexConfig`&gt;

#### 定義元 \{#defined-in\}

[server/search&#95;filter&#95;builder.ts:42](https://github.com/get-convex/convex-js/blob/main/src/server/search_filter_builder.ts#L42)