---
id: "server.VectorIndexConfig"
title: "インターフェース: VectorIndexConfig<VectorField, FilterFields>"
custom_edit_url: null
---

[server](../modules/server.md).VectorIndexConfig

ベクターインデックスの設定。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `VectorField` | extends `string` |
| `FilterFields` | extends `string` |

## プロパティ \{#properties\}

### vectorField \{#vectorfield\}

• **vectorField**: `VectorField`

ベクター検索のためにインデックスを作成するフィールドです。

これは `v.array(v.float64())` 型（またはその union 型）のフィールドである必要があります。

#### 定義場所 \{#defined-in\}

[server/schema.ts:123](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L123)

***

### dimensions \{#dimensions\}

• **dimensions**: `number`

インデックス化されるベクトルの次元数。2 以上 2048 以下である必要があります。

#### 定義元 \{#defined-in\}

[server/schema.ts:127](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L127)

***

### filterFields \{#filterfields\}

• `Optional` **filterFields**: `FilterFields`[]

ベクター検索を実行する際に、高速なフィルタリングを行うためにインデックス化される追加フィールド。

#### 定義元 \{#defined-in\}

[server/schema.ts:131](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L131)