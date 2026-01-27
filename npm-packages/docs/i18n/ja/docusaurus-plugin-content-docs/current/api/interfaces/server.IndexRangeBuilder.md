---
id: "server.IndexRangeBuilder"
title: "インターフェース: IndexRangeBuilder<Document, IndexFields, FieldNum>"
custom_edit_url: null
---

[server](../modules/server.md).IndexRangeBuilder

インデックス範囲を定義してクエリするためのビルダー。

インデックス範囲は、Convex がクエリを実行するときに
どのドキュメントを対象にするかを表す定義です。

インデックス範囲は常に、次のように連結された一連の条件になります:

1. `.eq` で定義された 0 個以上の等価式。
2. [オプション] `.gt` または `.gte` で定義された下限式。
3. [オプション] `.lt` または `.lte`で定義された上限式。

**フィールドは必ずインデックス順にたどる必要があります。**

それぞれの等価式は、先頭から順番に異なるインデックスフィールドを
比較しなければなりません。上限と下限は等価式の後に続き、
その次のフィールドを比較する必要があります。

たとえば、`["projectId", "priority"]` に対する messages のインデックスがあり、
「&#39;myProjectId&#39; のメッセージで priority が少なくとも 100」の範囲を
検索したい場合は、次のようになります:

```ts
q.eq("projectId", myProjectId)
 .gte("priority", 100)
```

**クエリのパフォーマンスは、指定する範囲がどれだけ絞り込まれているかに依存します。**

このクラスは、Convex がインデックスを効率的に利用して検索できる範囲のみを
指定できるように設計されています。それ以外のフィルタリングには
[filter](server.OrderedQuery.md#filter) を使用してください。

インデックスの詳細については、[Indexes](https://docs.convex.dev/using/indexes) を参照してください。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Document` | [`GenericDocument`](../modules/server.md#genericdocument) を継承 |
| `IndexFields` | [`GenericIndexFields`](../modules/server.md#genericindexfields) を継承 |
| `FieldNum` | `number` を継承し、既定値は `0` |

## 継承階層 \{#hierarchy\}

* `LowerBoundIndexRangeBuilder`&lt;`Document`, `IndexFields`[`FieldNum`]&gt;

  ↳ **`IndexRangeBuilder`**

## メソッド \{#methods\}

### eq \{#eq\}

▸ **eq**(`fieldName`, `value`): `NextIndexRangeBuilder`&lt;`Document`, `IndexFields`, `FieldNum`&gt;

この範囲を、`doc[fieldName] === value` であるドキュメントのみに絞り込みます。

#### パラメータ \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `fieldName` | `IndexFields`[`FieldNum`] | 比較するフィールドの名前。インデックス定義で直後のフィールドである必要があります。 |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `IndexFields`[`FieldNum`]&gt; | 比較に使用する値。 |

#### 戻り値 \{#returns\}

`NextIndexRangeBuilder`&lt;`Document`, `IndexFields`, `FieldNum`&gt;

#### 定義元 \{#defined-in\}

[server/index&#95;range&#95;builder.ts:76](https://github.com/get-convex/convex-js/blob/main/src/server/index_range_builder.ts#L76)

***

### gt \{#gt\}

▸ **gt**(`fieldName`, `value`): `UpperBoundIndexRangeBuilder`&lt;`Document`, `IndexFields`[`FieldNum`]&gt;

`doc[fieldName] > value` となるドキュメントにこの範囲を制限します。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `fieldName` | `IndexFields`[`FieldNum`] | 比較するフィールド名。インデックス内で次のフィールドである必要があります。 |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `IndexFields`[`FieldNum`]&gt; | 比較に使用する値。 |

#### 戻り値 \{#returns\}

`UpperBoundIndexRangeBuilder`&lt;`Document`, `IndexFields`[`FieldNum`]&gt;

#### 継承元 \{#inherited-from\}

LowerBoundIndexRangeBuilder.gt

#### 定義場所 \{#defined-in\}

[server/index&#95;range&#95;builder.ts:115](https://github.com/get-convex/convex-js/blob/main/src/server/index_range_builder.ts#L115)

***

### gte \{#gte\}

▸ **gte**(`fieldName`, `value`): `UpperBoundIndexRangeBuilder`&lt;`Document`, `IndexFields`[`FieldNum`]&gt;

この範囲を、`doc[fieldName] >= value` を満たすドキュメントのみに制限します。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `fieldName` | `IndexFields`[`FieldNum`] | 比較するフィールドの名前。インデックス内で次のフィールドである必要があります。 |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `IndexFields`[`FieldNum`]&gt; | 比較対象の値。 |

#### 戻り値 \{#returns\}

`UpperBoundIndexRangeBuilder`&lt;`Document`, `IndexFields`[`FieldNum`]&gt;

#### 継承元 \{#inherited-from\}

LowerBoundIndexRangeBuilder.gte

#### 定義場所 \{#defined-in\}

[server/index&#95;range&#95;builder.ts:126](https://github.com/get-convex/convex-js/blob/main/src/server/index_range_builder.ts#L126)

***

### lt \{#lt\}

▸ **lt**(`fieldName`, `value`): [`IndexRange`](../classes/server.IndexRange.md)

この範囲を、`doc[fieldName] < value` を満たすドキュメントのみに制限します。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `fieldName` | `IndexFields`[`FieldNum`] | 比較するフィールド名。下限（`.gt` または `.gte`）に使用したものと同じインデックスフィールド、または下限が指定されていない場合はその次のフィールドである必要があります。 |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `IndexFields`[`FieldNum`]&gt; | 比較対象の値。 |

#### 戻り値 \{#returns\}

[`IndexRange`](../classes/server.IndexRange.md)

#### 継承元 \{#inherited-from\}

LowerBoundIndexRangeBuilder.lt

#### 定義元 \{#defined-in\}

[server/index&#95;range&#95;builder.ts:151](https://github.com/get-convex/convex-js/blob/main/src/server/index_range_builder.ts#L151)

***

### lte \{#lte\}

▸ **lte**(`fieldName`, `value`): [`IndexRange`](../classes/server.IndexRange.md)

この範囲を、`doc[fieldName] <= value` を満たすドキュメントに絞り込みます。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `fieldName` | `IndexFields`[`FieldNum`] | 比較するフィールドの名前。下限（`.gt` または `.gte`）として使用したインデックスフィールドと同じであるか、下限が指定されていない場合はその次のフィールドである必要があります。 |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `IndexFields`[`FieldNum`]&gt; | 比較に使用する値。 |

#### 戻り値 \{#returns\}

[`IndexRange`](../classes/server.IndexRange.md)

#### 継承元 \{#inherited-from\}

LowerBoundIndexRangeBuilder.lte

#### 定義元 \{#defined-in\}

[server/index&#95;range&#95;builder.ts:164](https://github.com/get-convex/convex-js/blob/main/src/server/index_range_builder.ts#L164)