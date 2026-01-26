---
title: "dataModel.d.ts"
sidebar_position: 1
description: "为你的数据库模式和文档自动生成的 TypeScript 类型"
---

<Admonition type="caution" title="此代码为自动生成">
  这些导出在 `convex` 包中无法直接使用！

  你需要运行 `npx convex dev` 来创建
  `convex/_generated/dataModel.d.ts`。
</Admonition>

已生成的数据模型类型。

## 类型 \{#types\}

### TableNames \{#tablenames\}

Ƭ **TableNames**: `string`

你所有 Convex 数据表的名称。

***

### Doc \{#doc\}

Ƭ **Doc**`<TableName>`: `Object`

在 Convex 中存储的文档的类型。

#### 类型参数 \{#type-parameters\}

| 名称        | 类型                                | 描述                                                    |
| :---------- | :---------------------------------- | :------------------------------------------------------ |
| `TableName` | extends [`TableNames`](#tablenames) | 表名对应的字符串字面量类型（例如 &quot;users&quot;）。           |

***

### Id \{#id\}

Convex 中文档的标识符。

Convex 文档通过它们的 `Id` 唯一标识，该值可以从 `_id` 字段中访问。要了解更多信息，请参阅 [Document IDs](/database/document-ids.mdx)。

可以在查询和变更函数中使用 `db.get(tableName, id)` 来加载文档。

在运行时，Id 只是字符串，但通过这个类型，可以在类型检查时将它们与其他字符串区分开来。

这是一个针对你的数据模型设定类型的 [`GenericId`](/api/modules/values#genericid) 别名。

#### 类型参数 \{#type-parameters\}

| 名称        | 类型                                | 描述                                                     |
| :---------- | :---------------------------------- | :------------------------------------------------------ |
| `TableName` | extends [`TableNames`](#tablenames) | 表名的字符串字面量类型（例如 &quot;users&quot;）。 |

***

### DataModel \{#datamodel\}

Ƭ **DataModel**: `Object`

描述你的 Convex 数据模型的类型。

该类型包含关于你有哪些表、这些表中存储的文档类型，以及在这些表上定义的索引等信息。

此类型用于为 [`queryGeneric`](/api/modules/server#querygeneric) 和
[`mutationGeneric`](/api/modules/server#mutationgeneric) 之类的方法提供类型参数，从而保证它们的类型安全。