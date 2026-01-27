---
id: "server.FilterExpression"
title: "类：FilterExpression<T>"
custom_edit_url: null
---

[server](../modules/server.md).FilterExpression

在执行查询时，会对表达式求值以生成一个[值](../modules/values.md#value)。

要构造表达式，请使用 [VectorSearchQuery](../interfaces/server.VectorSearchQuery.md) 中提供的 [VectorFilterBuilder](../interfaces/server.VectorFilterBuilder.md)。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `T` | extends [`Value`](../modules/values.md#value) | `undefined` | 此表达式的求值结果类型。 |