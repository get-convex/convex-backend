---
id: "server.SearchFilterBuilder"
title: "接口：SearchFilterBuilder<Document, SearchIndexConfig>"
custom_edit_url: null
---

[server](../modules/server.md).SearchFilterBuilder

用于定义搜索过滤器的构建器。

搜索过滤器是一个链式调用序列，由以下部分组成：

1. 一个使用 `.search` 构造的搜索表达式。
2. 零个或多个使用 `.eq` 构造的等值表达式。

搜索表达式必须在索引的 `searchField` 中搜索文本。
过滤表达式可以使用索引中定义的任意 `filterFields`。

对于其他所有过滤需求，请使用 [filter](server.OrderedQuery.md#filter)。

要了解全文搜索，请参阅 [Indexes](https://docs.convex.dev/text-search)。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Document` | 继承自 [`GenericDocument`](../modules/server.md#genericdocument) |
| `SearchIndexConfig` | 继承自 [`GenericSearchIndexConfig`](../modules/server.md#genericsearchindexconfig) |

## 方法 \{#methods\}

### search \{#search\}

▸ **search**(`fieldName`, `query`): [`SearchFilterFinalizer`](server.SearchFilterFinalizer.md)&lt;`Document`, `SearchIndexConfig`&gt;

在 `doc[fieldName]` 中搜索 `query` 中的词项。

这会执行一次全文搜索，返回那些字段中包含 `query` 中任意单词的文档。

文档将根据它们与查询的相关性排序返回。这会考虑：

* 查询中有多少单词出现在文本中？
* 它们出现了多少次？
* 文本字段有多长？

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `fieldName` | `SearchIndexConfig`[`"searchField"`] | 要进行搜索的字段名称。它必须在该索引中被声明为 `searchField`。 |
| `query` | `string` | 要搜索的查询文本。 |

#### 返回值 \{#returns\}

[`SearchFilterFinalizer`](server.SearchFilterFinalizer.md)&lt;`Document`, `SearchIndexConfig`&gt;

#### 定义于 \{#defined-in\}

[server/search&#95;filter&#95;builder.ts:42](https://github.com/get-convex/convex-js/blob/main/src/server/search_filter_builder.ts#L42)