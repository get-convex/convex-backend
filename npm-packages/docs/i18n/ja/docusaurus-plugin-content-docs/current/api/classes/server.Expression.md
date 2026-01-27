---
id: "server.Expression"
title: "クラス: Expression<T>"
custom_edit_url: null
---

[server](../modules/server.md).Expression

式は、クエリの実行中に評価され、その結果として [Value](../modules/values.md#value) が生成されます。

式を構築するには、[filter](../interfaces/server.OrderedQuery.md#filter) 内で提供されている [FilterBuilder](../interfaces/server.FilterBuilder.md) を使用します。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `T` | extends [`Value`](../modules/values.md#value) | `undefined` | この式の評価結果の型。 |