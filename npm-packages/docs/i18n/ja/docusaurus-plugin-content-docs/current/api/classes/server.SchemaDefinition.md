---
id: "server.SchemaDefinition"
title: "クラス: SchemaDefinition<Schema, StrictTableTypes>"
custom_edit_url: null
---

[server](../modules/server.md).SchemaDefinition

Convex プロジェクトのスキーマ定義を表します。

これは [defineSchema](../modules/server.md#defineschema) を使って生成する必要があります。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Schema` | extends [`GenericSchema`](../modules/server.md#genericschema) |
| `StrictTableTypes` | extends `boolean` |

## プロパティ \{#properties\}

### tables \{#tables\}

• **tables**: `Schema`

#### 定義場所 \{#defined-in\}

[server/schema.ts:658](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L658)

***

### strictTableNameTypes \{#stricttablenametypes\}

• **strictTableNameTypes**: `StrictTableTypes`

#### 定義場所 \{#defined-in\}

[server/schema.ts:659](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L659)

***

### schemaValidation \{#schemavalidation\}

• `Readonly` **schemaValidation**: `boolean`

#### 定義元 \{#defined-in\}

[server/schema.ts:660](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L660)