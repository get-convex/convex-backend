---
id: "server.DefineSchemaOptions"
title: "接口：DefineSchemaOptions<StrictTableNameTypes>"
custom_edit_url: null
---

[server](../modules/server.md).DefineSchemaOptions

用于 [defineSchema](../modules/server.md#defineschema) 的选项。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `StrictTableNameTypes` | extends `boolean` |

## 属性 \{#properties\}

### schemaValidation \{#schemavalidation\}

• `Optional` **schemaValidation**: `boolean`

是否让 Convex 在运行时验证所有文档都符合你的模式。

如果 `schemaValidation` 为 `true`，Convex 将：

1. 在推送你的模式时，检查所有现有文档是否符合该模式。
2. 在变更期间，检查所有插入和更新是否符合该模式。

如果 `schemaValidation` 为 `false`，Convex 将不会验证新文档或
现有文档是否符合你的模式。你仍然会获得基于模式生成的
TypeScript 类型，但在运行时不会有任何验证来确保你的
文档符合这些类型。

默认情况下，`schemaValidation` 为 `true`。

#### 定义于 \{#defined-in\}

[server/schema.ts:727](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L727)

***

### strictTableNameTypes \{#stricttablenametypes\}

• `Optional` **strictTableNameTypes**: `StrictTableNameTypes`

是否允许通过 TypeScript 类型访问不在模式中的数据表。

如果 `strictTableNameTypes` 为 `true`，使用未在模式中列出的数据表
将导致 TypeScript 编译错误。

如果 `strictTableNameTypes` 为 `false`，你将能够访问未在模式中列出的数据表，
并且这些表中文档的类型将是 `any`。

在快速原型开发中，使用 `strictTableNameTypes: false` 会很有帮助。

无论 `strictTableNameTypes` 的取值如何，你的模式都只会验证
在模式中列出的数据表中的文档。你仍然可以在仪表盘或 JavaScript 变更函数中
创建和修改其他数据表。

默认情况下，`strictTableNameTypes` 为 `true`。

#### 定义于 \{#defined-in\}

[server/schema.ts:746](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L746)