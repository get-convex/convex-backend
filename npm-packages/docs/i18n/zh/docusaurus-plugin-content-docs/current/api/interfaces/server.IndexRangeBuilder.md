---
id: "server.IndexRangeBuilder"
title: "接口：IndexRangeBuilder<Document, IndexFields, FieldNum>"
custom_edit_url: null
---

[server](../modules/server.md).IndexRangeBuilder

用于构建要进行查询的索引范围的构造器。

索引范围用于描述在运行查询时 Convex 应该考虑哪些文档。

索引范围始终是以下内容的链式列表：

1. 使用 `.eq` 定义的 0 个或多个相等表达式。
2. [可选] 使用 `.gt` 或 `.gte` 定义的下界表达式。
3. [可选] 使用 `.lt` 或 `.lte` 定义的上界表达式。

**你必须按照索引顺序逐个字段地指定条件。**

每个相等表达式必须从头开始，依次比较不同的索引字段。上下界必须紧随这些相等表达式之后，并比较下一个字段。

例如，如果有一个针对 `messages` 的索引，其字段为
`["projectId", "priority"]`，用于搜索 &quot;项目 &#39;myProjectId&#39; 中优先级至少为 100 的消息&quot; 的范围将会是：

```ts
q.eq("projectId", myProjectId)
 .gte("priority", 100)
```

**查询的性能取决于范围的精确程度。**

此类只允许你指定 Convex 能够高效利用索引来查找的范围。对于所有其他筛选，请使用
[filter](server.OrderedQuery.md#filter)。

要了解索引，请参阅 [Indexes](https://docs.convex.dev/using/indexes)。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Document` | 继承自 [`GenericDocument`](../modules/server.md#genericdocument) |
| `IndexFields` | 继承自 [`GenericIndexFields`](../modules/server.md#genericindexfields) |
| `FieldNum` | 继承自 `number` = `0` |

## 层次结构 \{#hierarchy\}

* `LowerBoundIndexRangeBuilder`&lt;`Document`, `IndexFields`[`FieldNum`]&gt;

  ↳ **`IndexRangeBuilder`**

## 方法 \{#methods\}

### eq \{#eq\}

▸ **eq**(`fieldName`, `value`): `NextIndexRangeBuilder`&lt;`Document`, `IndexFields`, `FieldNum`&gt;

将当前范围限制为满足 `doc[fieldName] === value` 的文档。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `fieldName` | `IndexFields`[`FieldNum`] | 要比较的字段名称。必须是索引中的下一个字段。 |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `IndexFields`[`FieldNum`]&gt; | 用来比较的值。 |

#### 返回 \{#returns\}

`NextIndexRangeBuilder`&lt;`Document`, `IndexFields`, `FieldNum`&gt;

#### 定义于 \{#defined-in\}

[server/index&#95;range&#95;builder.ts:76](https://github.com/get-convex/convex-js/blob/main/src/server/index_range_builder.ts#L76)

***

### gt \{#gt\}

▸ **gt**(`fieldName`, `value`): `UpperBoundIndexRangeBuilder`&lt;`Document`, `IndexFields`[`FieldNum`]&gt;

将此范围限制为仅包含满足 `doc[fieldName] > value` 的文档。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `fieldName` | `IndexFields`[`FieldNum`] | 要比较的字段名。必须是索引中的下一个字段。 |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `IndexFields`[`FieldNum`]&gt; | 要与之比较的值。 |

#### 返回值 \{#returns\}

`UpperBoundIndexRangeBuilder`&lt;`Document`, `IndexFields`[`FieldNum`]&gt;

#### 继承自 \{#inherited-from\}

LowerBoundIndexRangeBuilder.gt

#### 定义于 \{#defined-in\}

[server/index&#95;range&#95;builder.ts:115](https://github.com/get-convex/convex-js/blob/main/src/server/index_range_builder.ts#L115)

***

### gte \{#gte\}

▸ **gte**(`fieldName`, `value`): `UpperBoundIndexRangeBuilder`&lt;`Document`, `IndexFields`[`FieldNum`]&gt;

将此范围限定为满足 `doc[fieldName] >= value` 的文档。

#### 参数 \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `fieldName` | `IndexFields`[`FieldNum`] | 要比较的字段名。必须是索引中的下一个字段。 |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `IndexFields`[`FieldNum`]&gt; | 用来进行比较的值。 |

#### 返回 \{#returns\}

`UpperBoundIndexRangeBuilder`&lt;`Document`, `IndexFields`[`FieldNum`]&gt;

#### 继承自 \{#inherited-from\}

LowerBoundIndexRangeBuilder.gte

#### 定义于 \{#defined-in\}

[server/index&#95;range&#95;builder.ts:126](https://github.com/get-convex/convex-js/blob/main/src/server/index_range_builder.ts#L126)

***

### lt \{#lt\}

▸ **lt**(`fieldName`, `value`): [`IndexRange`](../classes/server.IndexRange.md)

将此范围限制为仅包含满足 `doc[fieldName] &lt; value` 的文档。

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `fieldName` | `IndexFields`[`FieldNum`] | 要比较的字段名。必须与下界（`.gt` 或 `.gte`）中使用的同一索引字段相同，或者在未指定下界时为下一个字段。 |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `IndexFields`[`FieldNum`]&gt; | 要用于比较的值。 |

#### 返回 \{#returns\}

[`IndexRange`](../classes/server.IndexRange.md)

#### 继承自 \{#inherited-from\}

LowerBoundIndexRangeBuilder.lt

#### 定义于 \{#defined-in\}

[server/index&#95;range&#95;builder.ts:151](https://github.com/get-convex/convex-js/blob/main/src/server/index_range_builder.ts#L151)

***

### lte \{#lte\}

▸ **lte**(`fieldName`, `value`): [`IndexRange`](../classes/server.IndexRange.md)

将此范围限制为仅包含满足 `doc[fieldName] <= value` 的文档。

#### 参数 \{#parameters\}

| 名称 | 类型 | 说明 |
| :------ | :------ | :------ |
| `fieldName` | `IndexFields`[`FieldNum`] | 要比较的字段名。必须与下界（`.gt` 或 `.gte`）中使用的同一索引字段相同，或者在未指定下界时为下一个字段。 |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `IndexFields`[`FieldNum`]&gt; | 要用于比较的值。 |

#### 返回值 \{#returns\}

[`IndexRange`](../classes/server.IndexRange.md)

#### 继承自 \{#inherited-from\}

LowerBoundIndexRangeBuilder.lte

#### 定义于 \{#defined-in\}

[server/index&#95;range&#95;builder.ts:164](https://github.com/get-convex/convex-js/blob/main/src/server/index_range_builder.ts#L164)