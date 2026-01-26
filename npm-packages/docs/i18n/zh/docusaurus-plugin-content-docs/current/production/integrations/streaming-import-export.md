---
title: "在 Convex 中进行数据流式导入与导出"
sidebar_label: "流式导入/导出"
description: "在 Convex 中进行数据流式导入与导出"
sidebar_position: 4
---

[Fivetran](https://www.fivetran.com) 和 [Airbyte](https://airbyte.com) 是数据集成平台，允许你将 Convex 数据与其他数据库进行同步。

Fivetran 支持将 Convex 的数据以流式方式导出到其任一
[受支持的目标端](https://fivetran.com/docs/destinations)。Convex 团队维护了一个 Convex 源连接器，用于流式导出。目前暂不支持通过 Fivetran 将数据流式导入 Convex。

使用 Airbyte，你可以将其任一
[受支持的来源](https://airbyte.com/connectors?connector-type=Sources) 的数据流式导入 Convex，并将 Convex 的数据流式导出到其任一
[受支持的目标端](https://airbyte.com/connectors?connector-type=Destinations)。
Convex 团队维护了一个用于流式导出的 Convex 源连接器，以及一个用于流式导入的 Convex 目标连接器。

<BetaAdmonition feature="Fivetran & Airbyte integrations" verb="are" />

## 流式导出 \{#streaming-export\}

导出数据对于处理 Convex 不能直接支持的工作负载非常有用。一些常见用例包括：

1. 数据分析
   * Convex 并未针对加载海量数据的查询进行优化。像 [Databricks](https://www.databricks.com) 或
     [Snowflake](https://www.snowflake.com/) 这样的数据平台更为合适。
2. 灵活查询
   * 虽然 Convex 提供了强大的
     [数据库查询](/database/reading-data/reading-data.mdx#querying-documents)
     和内置的[全文搜索](/search.mdx)支持，但仍然有一些在 Convex 中难以编写的查询。如果你需要为类似“高级搜索”视图提供高度动态的排序和过滤功能，
     像 [ElasticSearch](https://www.elastic.co) 这样的数据库会很有帮助。
3. 机器学习训练
   * Convex 并未针对运行计算密集型机器学习算法的查询进行优化。

<ProFeatureUpsell feature="流式导出" verb="需要" />

参阅 [Fivetran](https://fivetran.com/integrations/convex) 或
[Airbyte](https://docs.airbyte.com/integrations/sources/convex) 文档，了解如何设置流式导出。如果你需要帮助或有任何问题，请[联系我们](https://convex.dev/community)。

## 流式导入 \{#streaming-import\}

采用新技术可能是一个缓慢且艰巨的过程，尤其是当这些技术涉及数据库时。流式导入可以让你在不必编写自己的迁移或数据
同步工具的情况下，将 Convex 与你现有的技术栈一同使用。一些典型用例包括：

1. 使用你项目自身的数据制作原型，验证 Convex 如何替换你项目现有的后端。
2. 在保留现有数据库的同时使用 Convex，更快速地构建新产品。
3. 在现有数据集之上开发一个响应式的 UI 层。
4. 将你的数据迁移到 Convex（如果 [CLI](/cli.md) 工具不能满足你的需求）。

<Admonition type="caution" title="将导入的表设为只读">
  一个常见用例是将源数据库中的某个表“镜像”到 Convex 中，以便使用 Convex 构建新的功能。我们建议在 Convex 中将导入的
  表保持为只读，因为把结果同步回源数据库可能会导致危险的写入冲突。虽然 Convex 目前还没有能够保证某个表只读的访问控制，
  但你可以确保你的代码中没有向导入表写入的变更或操作函数，并避免在仪表盘中编辑导入表里的文档。
</Admonition>

流式导入包含在所有 Convex 方案中。要了解如何配置 Convex 目标连接器，请查看 Airbyte 文档，
参见[此处](https://docs.airbyte.com/integrations/destinations/convex)。