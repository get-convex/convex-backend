---
id: "server.VectorFilterBuilder"
title: "接口：VectorFilterBuilder<Document, VectorIndexConfig>"
custom_edit_url: null
---

[server](../modules/server.md).VectorFilterBuilder

用于为向量搜索定义筛选条件的接口。

它的接口与用于数据库查询的 [FilterBuilder](server.FilterBuilder.md) 类似，但只支持那些可以在向量搜索中高效执行的方法。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Document` | extends [`GenericDocument`](../modules/server.md#genericdocument) |
| `VectorIndexConfig` | extends [`GenericVectorIndexConfig`](../modules/server.md#genericvectorindexconfig) |

## 方法 \{#methods\}

### eq \{#eq\}

▸ **eq**&lt;`FieldName`&gt;(`fieldName`, `value`): [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

`fieldName` 指定的字段是否等于 `value`

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `FieldName` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `fieldName` | `FieldName` |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `FieldName`&gt; |

#### 返回 \{#returns\}

[`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

#### 定义于 \{#defined-in\}

[server/vector&#95;search.ts:110](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L110)

***

### or \{#or\}

▸ **or**(`...exprs`): [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

`exprs[0] || exprs[1] || ... || exprs[n]`

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `...exprs` | [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;[] |

#### 返回值 \{#returns\}

[`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

#### 定义于 \{#defined-in\}

[server/vector&#95;search.ts:122](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L122)