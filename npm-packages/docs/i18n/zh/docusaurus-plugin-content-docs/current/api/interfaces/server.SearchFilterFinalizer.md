---
id: "server.SearchFilterFinalizer"
title: "接口：SearchFilterFinalizer&lt;Document, SearchIndexConfig&gt;"
custom_edit_url: null
---

[server](../modules/server.md).SearchFilterFinalizer

用于在搜索过滤器中定义相等条件表达式的构建器。

参见 [SearchFilterBuilder](server.SearchFilterBuilder.md)。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Document` | 继承自 [`GenericDocument`](../modules/server.md#genericdocument) |
| `SearchIndexConfig` | 继承自 [`GenericSearchIndexConfig`](../modules/server.md#genericsearchindexconfig) |

## 层次结构 \{#hierarchy\}

* [`SearchFilter`](../classes/server.SearchFilter.md)

  ↳ **`SearchFilterFinalizer`**

## 方法 \{#methods\}

### eq \{#eq\}

▸ **eq**&lt;`FieldName`&gt;(`fieldName`, `value`): [`SearchFilterFinalizer`](server.SearchFilterFinalizer.md)&lt;`Document`, `SearchIndexConfig`&gt;

将此查询限制为仅返回满足 `doc[fieldName] === value` 的文档。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `FieldName` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `fieldName` | `FieldName` | 要比较的字段名称。该字段必须出现在搜索索引的 `filterFields` 中。 |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `FieldName`&gt; | 用于比较的值。 |

#### 返回 \{#returns\}

[`SearchFilterFinalizer`](server.SearchFilterFinalizer.md)&lt;`Document`, `SearchIndexConfig`&gt;

#### 定义于 \{#defined-in\}

[server/search&#95;filter&#95;builder.ts:66](https://github.com/get-convex/convex-js/blob/main/src/server/search_filter_builder.ts#L66)