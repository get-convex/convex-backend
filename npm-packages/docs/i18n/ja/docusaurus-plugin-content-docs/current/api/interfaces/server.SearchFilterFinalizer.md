---
id: "server.SearchFilterFinalizer"
title: "インターフェース: SearchFilterFinalizer<Document, SearchIndexConfig>"
custom_edit_url: null
---

[server](../modules/server.md).SearchFilterFinalizer

検索フィルターの一部として等値条件を定義するためのビルダーです。

[SearchFilterBuilder](server.SearchFilterBuilder.md) を参照してください。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Document` | [`GenericDocument`](../modules/server.md#genericdocument) を拡張 |
| `SearchIndexConfig` | [`GenericSearchIndexConfig`](../modules/server.md#genericsearchindexconfig) を拡張 |

## 継承階層 \{#hierarchy\}

* [`SearchFilter`](../classes/server.SearchFilter.md)

  ↳ **`SearchFilterFinalizer`**

## メソッド \{#methods\}

### eq \{#eq\}

▸ **eq**&lt;`FieldName`&gt;(`fieldName`, `value`): [`SearchFilterFinalizer`](server.SearchFilterFinalizer.md)&lt;`Document`, `SearchIndexConfig`&gt;

このクエリを、`doc[fieldName] === value` となるドキュメントのみに絞り込みます。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `FieldName` | extends `string` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `fieldName` | `FieldName` | 比較するフィールドの名前。検索インデックスの `filterFields` に含まれている必要があります。 |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `FieldName`&gt; | 比較対象の値。 |

#### 戻り値 \{#returns\}

[`SearchFilterFinalizer`](server.SearchFilterFinalizer.md)&lt;`Document`, `SearchIndexConfig`&gt;

#### 定義場所 \{#defined-in\}

[server/search&#95;filter&#95;builder.ts:66](https://github.com/get-convex/convex-js/blob/main/src/server/search_filter_builder.ts#L66)