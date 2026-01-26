---
id: "server.VectorIndexConfig"
title: "接口：VectorIndexConfig<VectorField, FilterFields>"
custom_edit_url: null
---

[server](../modules/server.md).VectorIndexConfig

向量索引的配置。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `VectorField` | extends `string` |
| `FilterFields` | extends `string` |

## 属性 \{#properties\}

### vectorField \{#vectorfield\}

• **vectorField**: `VectorField`

要为向量搜索建立索引的字段。

此字段的类型必须是 `v.array(v.float64())`（或该类型的 union）

#### 定义于 \{#defined-in\}

[server/schema.ts:123](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L123)

***

### dimensions \{#dimensions\}

• **dimensions**: `number`

索引中向量的维度数。必须在 2 到 2048（含）之间。

#### 定义于 \{#defined-in\}

[server/schema.ts:127](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L127)

***

### filterFields \{#filterfields\}

• `Optional` **filterFields**: `FilterFields`[]

在运行向量搜索时，为实现快速过滤而额外建立索引的字段。

#### 定义于 \{#defined-in\}

[server/schema.ts:131](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L131)