---
id: "server.FilterExpression"
title: "クラス: FilterExpression<T>"
custom_edit_url: null
---

[server](../modules/server.md).FilterExpression

式は、クエリの実行中に[値](../modules/values.md#value)を生成するために評価されます。

式を構築するには、
[VectorSearchQuery](../interfaces/server.VectorSearchQuery.md) 内で提供されている [VectorFilterBuilder](../interfaces/server.VectorFilterBuilder.md) を使用します。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `T` | extends [`Value`](../modules/values.md#value) | `undefined` | この式の評価結果の型。 |