---
id: "server.VectorFilterBuilder"
title: "インターフェース: VectorFilterBuilder<Document, VectorIndexConfig>"
custom_edit_url: null
---

[server](../modules/server.md).VectorFilterBuilder

ベクター検索用のフィルターを定義するためのインターフェースです。

これはデータベースのクエリで使用される [FilterBuilder](server.FilterBuilder.md) と
類似したインターフェースですが、ベクター検索で効率的に
実行できるメソッドのみをサポートします。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Document` | extends [`GenericDocument`](../modules/server.md#genericdocument) |
| `VectorIndexConfig` | extends [`GenericVectorIndexConfig`](../modules/server.md#genericvectorindexconfig) |

## メソッド \{#methods\}

### eq \{#eq\}

▸ **eq**&lt;`FieldName`&gt;(`fieldName`, `value`): [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

フィールド `fieldName` が `value` と等しいかどうかを判定します

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `FieldName` | extends `string` |

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `fieldName` | `FieldName` |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `FieldName`&gt; |

#### 戻り値 \{#returns\}

[`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

#### 定義場所 \{#defined-in\}

[server/vector&#95;search.ts:110](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L110)

***

### or \{#or\}

▸ **or**(`...exprs`): [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

`exprs[0] || exprs[1] || ... || exprs[n]`

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `...exprs` | [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;[] |

#### 戻り値 \{#returns\}

[`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

#### 定義場所 \{#defined-in\}

[server/vector&#95;search.ts:122](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L122)