---
id: "server.SearchIndexConfig"
title: "接口：SearchIndexConfig<SearchField, FilterFields>"
custom_edit_url: null
---

[server](../modules/server.md).SearchIndexConfig

全文本搜索索引的配置。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `SearchField` | extends `string` |
| `FilterFields` | extends `string` |

## 属性 \{#properties\}

### searchField \{#searchfield\}

• **searchField**: `SearchField`

用于全文搜索索引的字段。

该字段的类型必须是 `string`。

#### 定义于 \{#defined-in\}

[server/schema.ts:101](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L101)

***

### filterFields \{#filterfields\}

• `Optional` **filterFields**: `FilterFields`[]

用于建立索引的附加字段，在运行搜索查询时实现快速过滤。

#### 定义于 \{#defined-in\}

[server/schema.ts:106](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L106)