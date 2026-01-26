---
id: "server.FilterExpression"
title: "Clase: FilterExpression<T>"
custom_edit_url: null
---

[server](../modules/server.md).FilterExpression

Las expresiones se evalúan para producir un [Valor](../modules/values.md#value) durante la ejecución de una consulta.

Para construir una expresión, usa el [VectorFilterBuilder](../interfaces/server.VectorFilterBuilder.md) proporcionado en
[VectorSearchQuery](../interfaces/server.VectorSearchQuery.md).

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `T` | extends [`Value`](../modules/values.md#value) | `undefined` | El tipo al que se evalúa esta expresión. |