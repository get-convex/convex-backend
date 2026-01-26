---
id: "server.Expression"
title: "类：Expression<T>"
custom_edit_url: null
---

[server](../modules/server.md).Expression

表达式在执行查询时会被求值，以生成一个[值](../modules/values.md#value)。

要构造表达式，请使用 [filter](../interfaces/server.OrderedQuery.md#filter) 中提供的 [FilterBuilder](../interfaces/server.FilterBuilder.md)。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `T` | extends [`Value`](../modules/values.md#value) | `undefined` | 此表达式的求值结果类型。 |