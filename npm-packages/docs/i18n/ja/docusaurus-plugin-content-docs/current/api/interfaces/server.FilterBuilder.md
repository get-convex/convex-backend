---
id: "server.FilterBuilder"
title: "インターフェース: FilterBuilder<TableInfo>"
custom_edit_url: null
---

[server](../modules/server.md).FilterBuilder

クエリ内でフィルターを定義するためのインターフェース。

`FilterBuilder` には、さまざまな [Expression](../classes/server.Expression.md) を生成するメソッドがあります。
これらの式は定数と共に入れ子にして組み合わせることで、
フィルター条件（述語）を表現できます。

`FilterBuilder` は [filter](server.OrderedQuery.md#filter) 内で使用され、クエリの
フィルターを作成します。

利用可能なメソッドは次のとおりです:

|                               |                                               |
|-------------------------------|-----------------------------------------------|
| **比較**                      | `l` と `r` の型が同じでない場合はエラーになります。 |
| [`eq(l, r)`](#eq)             | `l === r`                                     |
| [`neq(l, r)`](#neq)           | `l !== r`                                     |
| [`lt(l, r)`](#lt)             | `l < r`                                       |
| [`lte(l, r)`](#lte)           | `l <= r`                                      |
| [`gt(l, r)`](#gt)             | `l > r`                                       |
| [`gte(l, r)`](#gte)           | `l >= r`                                      |
|                               |                                               |
| **算術**                      | `l` と `r` の型が同じでない場合はエラーになります。 |
| [`add(l, r)`](#add)           | `l + r`                                       |
| [`sub(l, r)`](#sub)           | `l - r`                                       |
| [`mul(l, r)`](#mul)           | `l * r`                                       |
| [`div(l, r)`](#div)           | `l / r`                                       |
| [`mod(l, r)`](#mod)           | `l % r`                                       |
| [`neg(x)`](#neg)              | `-x`                                          |
|                               |                                               |
| **論理**                      | いずれかのパラメータが `bool` でない場合はエラーになります。 |
| [`not(x)`](#not)              | `!x`                                          |
| [`and(a, b, ..., z)`](#and)   | `a && b && ... && z`                          |
| [`or(a, b, ..., z)`](#or)     | <code>a &#124;&#124; b &#124;&#124; ... &#124;&#124; z</code> |
|                               |                                               |
| **その他**                    |                                               |
| [`field(fieldPath)`](#field)  | `fieldPath` で指定されたフィールドを評価します。 |

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableInfo` | extends [`GenericTableInfo`](../modules/server.md#generictableinfo) |

## メソッド \{#methods\}

### eq \{#eq\}

▸ **eq**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l === r`

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends `undefined` | [`Value`](../modules/values.md#value) |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 戻り値 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定義場所 \{#defined-in\}

[server/filter&#95;builder.ts:87](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L87)

***

### neq \{#neq\}

▸ **neq**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l !== r`

#### 型パラメーター \{#type-parameters\}

| パラメーター名 | 型 |
| :------ | :------ |
| `T` | extends `undefined` | [`Value`](../modules/values.md#value) |

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 戻り値 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定義場所 \{#defined-in\}

[server/filter&#95;builder.ts:97](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L97)

***

### lt \{#lt\}

▸ **lt**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l < r`

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends [`Value`](../modules/values.md#value) |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 戻り値 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定義元 \{#defined-in\}

[server/filter&#95;builder.ts:107](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L107)

***

### lte \{#lte\}

▸ **lte**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l <= r`

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends [`値`](../modules/values.md#value) |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 戻り値 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定義先 \{#defined-in\}

[server/filter&#95;builder.ts:117](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L117)

***

### gt \{#gt\}

▸ **gt**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l > r`

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | [`値`](../modules/values.md#value) を継承する |

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 戻り値 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定義場所 \{#defined-in\}

[server/filter&#95;builder.ts:127](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L127)

***

### gte \{#gte\}

▸ **gte**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l >= r`

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends [`Value`](../modules/values.md#value) |

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 戻り値 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定義場所 \{#defined-in\}

[server/filter&#95;builder.ts:137](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L137)

***

### add \{#add\}

▸ **add**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`l + r`

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends [`NumericValue`](../modules/values.md#numericvalue) |

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 戻り値 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### 定義場所 \{#defined-in\}

[server/filter&#95;builder.ts:149](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L149)

***

### sub \{#sub\}

▸ **sub**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`l - r`

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | [`NumericValue`](../modules/values.md#numericvalue) を継承する型 |

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 戻り値 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### 定義箇所 \{#defined-in\}

[server/filter&#95;builder.ts:159](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L159)

***

### mul \{#mul\}

▸ **mul**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`l * r`

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends [`NumericValue`](../modules/values.md#numericvalue) |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 戻り値 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### 定義場所 \{#defined-in\}

[server/filter&#95;builder.ts:169](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L169)

***

### div \{#div\}

▸ **div**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`l / r`

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends [`NumericValue`](../modules/values.md#numericvalue) |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 戻り値 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### 定義場所 \{#defined-in\}

[server/filter&#95;builder.ts:179](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L179)

***

### mod \{#mod\}

▸ **mod**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`l % r`

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | [`NumericValue`](../modules/values.md#numericvalue) を拡張する型 |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 戻り値 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### 定義場所 \{#defined-in\}

[server/filter&#95;builder.ts:189](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L189)

***

### neg \{#neg\}

▸ **neg**&lt;`T`&gt;(`x`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`-x`

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends [`NumericValue`](../modules/values.md#numericvalue) |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `x` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 戻り値 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### 定義元 \{#defined-in\}

[server/filter&#95;builder.ts:199](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L199)

***

### and \{#and\}

▸ **and**(`...exprs`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`exprs[0] && exprs[1] && ... && exprs[n]`

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `...exprs` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt;[] |

#### 戻り値 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定義場所 \{#defined-in\}

[server/filter&#95;builder.ts:208](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L208)

***

### or \{#or\}

▸ **or**(`...exprs`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`exprs[0] || exprs[1] || ... || exprs[n]`

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `...exprs` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt;[] |

#### 戻り値 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定義元 \{#defined-in\}

[server/filter&#95;builder.ts:215](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L215)

***

### not \{#not\}

▸ **not**(`x`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`!x`

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `x` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt; |

#### 戻り値 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定義場所 \{#defined-in\}

[server/filter&#95;builder.ts:222](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L222)

***

### field \{#field\}

▸ **field**&lt;`FieldPath`&gt;(`fieldPath`): [`Expression`](../classes/server.Expression.md)&lt;[`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `FieldPath`&gt;&gt;

指定された `fieldPath` のフィールドを表す式に評価されます。

たとえば [filter](server.OrderedQuery.md#filter) 内で、フィルタリング対象の値を参照するために使用できます。

#### 例 \{#example\}

このオブジェクトに対しては次のようになります:

```
{
  "user": {
    "isActive": true
  }
}
```

`field("user.isActive")` は `true` と評価されます。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `FieldPath` | extends `string` |

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `fieldPath` | `FieldPath` |

#### 戻り値 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;[`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `FieldPath`&gt;&gt;

#### 定義場所 \{#defined-in\}

[server/filter&#95;builder.ts:246](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L246)