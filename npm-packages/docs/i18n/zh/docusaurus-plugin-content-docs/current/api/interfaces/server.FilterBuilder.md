---
id: "server.FilterBuilder"
title: "接口：FilterBuilder<TableInfo>"
custom_edit_url: null
---

[server](../modules/server.md).FilterBuilder

用于在查询中定义过滤器的接口。

`FilterBuilder` 拥有多种方法，可生成 [Expression](../classes/server.Expression.md) 表达式。
这些表达式可以与常量一起嵌套组合，用于表示
过滤谓词。

`FilterBuilder` 在 [filter](server.OrderedQuery.md#filter) 中使用，用于创建查询
过滤条件。

可用的方法如下：

|                               |                                               |
|-------------------------------|-----------------------------------------------|
| **比较**                      | 当 `l` 和 `r` 不是同一类型时出错。            |
| [`eq(l, r)`](#eq)             | `l === r`                                     |
| [`neq(l, r)`](#neq)           | `l !== r`                                     |
| [`lt(l, r)`](#lt)             | `l < r`                                       |
| [`lte(l, r)`](#lte)           | `l <= r`                                      |
| [`gt(l, r)`](#gt)             | `l > r`                                       |
| [`gte(l, r)`](#gte)           | `l >= r`                                      |
|                               |                                               |
| **算术运算**                  | 当 `l` 和 `r` 不是同一类型时出错。            |
| [`add(l, r)`](#add)           | `l + r`                                       |
| [`sub(l, r)`](#sub)           | `l - r`                                       |
| [`mul(l, r)`](#mul)           | `l * r`                                       |
| [`div(l, r)`](#div)           | `l / r`                                       |
| [`mod(l, r)`](#mod)           | `l % r`                                       |
| [`neg(x)`](#neg)              | `-x`                                          |
|                               |                                               |
| **逻辑运算**                  | 如果任一参数不是 `bool` 则出错。              |
| [`not(x)`](#not)              | `!x`                                          |
| [`and(a, b, ..., z)`](#and)   | `a && b && ... && z`                          |
| [`or(a, b, ..., z)`](#or)     | <code>a &#124;&#124; b &#124;&#124; ... &#124;&#124; z</code> |
|                               |                                               |
| **其他**                      |                                               |
| [`field(fieldPath)`](#field)  | 求值为 `fieldPath` 对应的字段。               |

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableInfo` | 继承自 [`GenericTableInfo`](../modules/server.md#generictableinfo) |

## 方法 \{#methods\}

### eq \{#eq\}

▸ **eq**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l === r`

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | extends `undefined` | [`Value`](../modules/values.md#value) |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 返回 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定义于 \{#defined-in\}

[server/filter&#95;builder.ts:87](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L87)

***

### neq \{#neq\}

▸ **neq**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l !== r`

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | 约束为 `undefined` | [`值`](../modules/values.md#value) |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 返回值 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定义于 \{#defined-in\}

[server/filter&#95;builder.ts:97](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L97)

***

### lt \{#lt\}

▸ **lt**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l < r`

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | 扩展自 [`值`](../modules/values.md#value) 类型 |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 返回值 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定义在 \{#defined-in\}

[server/filter&#95;builder.ts:107](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L107)

***

### lte \{#lte\}

▸ **lte**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l <= r`

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | 扩展自 [`Value`](../modules/values.md#value) |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 返回值 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定义在 \{#defined-in\}

[server/filter&#95;builder.ts:117](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L117)

***

### gt \{#gt\}

▸ **gt**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l > r`

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | 扩展自 [`Value`](../modules/values.md#value) |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 返回值 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定义于 \{#defined-in\}

[server/filter&#95;builder.ts:127](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L127)

***

### gte \{#gte\}

▸ **gte**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l >= r`

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | extends [`Value`](../modules/values.md#value) |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 返回 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定义于 \{#defined-in\}

[server/filter&#95;builder.ts:137](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L137)

***

### add \{#add\}

▸ **add**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`l + r`

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | 继承自 [`NumericValue`](../modules/values.md#numericvalue) |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 返回值 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### 定义于 \{#defined-in\}

[server/filter&#95;builder.ts:149](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L149)

***

### sub \{#sub\}

▸ **sub**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`l - r`

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | 受限于 [`NumericValue`](../modules/values.md#numericvalue) |

#### 参数 \{#parameters\}

| 参数名 | 类型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 返回值 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### 定义于 \{#defined-in\}

[server/filter&#95;builder.ts:159](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L159)

***

### mul \{#mul\}

▸ **mul**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`l * r`

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | 约束为 [`NumericValue`](../modules/values.md#numericvalue) |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 返回值 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### 定义于 \{#defined-in\}

[server/filter&#95;builder.ts:169](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L169)

***

### div \{#div\}

▸ **div**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`l / r`

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | 约束为 [`NumericValue`](../modules/values.md#numericvalue) |

#### 参数 \{#parameters\}

| 参数名 | 类型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 返回值 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### 定义于 \{#defined-in\}

[server/filter&#95;builder.ts:179](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L179)

***

### mod \{#mod\}

▸ **mod**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`l % r`

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | 受限于 [`NumericValue`](../modules/values.md#numericvalue) |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 返回 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### 定义于 \{#defined-in\}

[server/filter&#95;builder.ts:189](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L189)

***

### neg \{#neg\}

▸ **neg**&lt;`T`&gt;(`x`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`-x`

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | 扩展自 [`NumericValue`](../modules/values.md#numericvalue) |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `x` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### 返回 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### 定义于 \{#defined-in\}

[server/filter&#95;builder.ts:199](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L199)

***

### and \{#and\}

▸ **and**(`...exprs`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`exprs[0] && exprs[1] && ... && exprs[n]`

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `...exprs` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt;[] |

#### 返回值 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定义于 \{#defined-in\}

[server/filter&#95;builder.ts:208](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L208)

***

### or \{#or\}

▸ **or**(`...exprs`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`exprs[0] || exprs[1] || ... || exprs[n]`

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `...exprs` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt;[] |

#### 返回值 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定义于 \{#defined-in\}

[server/filter&#95;builder.ts:215](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L215)

***

### not \{#not\}

▸ **not**(`x`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`!x`

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `x` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt; |

#### 返回值 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### 定义于 \{#defined-in\}

[server/filter&#95;builder.ts:222](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L222)

***

### field \{#field\}

▸ **field**&lt;`FieldPath`&gt;(`fieldPath`): [`Expression`](../classes/server.Expression.md)&lt;[`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `FieldPath`&gt;&gt;

求值为给定 `fieldPath` 对应的字段。

例如，在 [filter](server.OrderedQuery.md#filter) 中，可以使用它来检查参与过滤的值。

#### 示例 \{#example\}

对于这个对象：

```
{
  "user": {
    "isActive": true
  }
}
```

`field("user.isActive")` 的求值结果为 `true`。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `FieldPath` | 受限于 `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `fieldPath` | `FieldPath` |

#### 返回 \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;[`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `FieldPath`&gt;&gt;

#### 定义于 \{#defined-in\}

[server/filter&#95;builder.ts:246](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L246)