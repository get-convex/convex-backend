---
id: "server.Expression"
title: "Clase: Expression<T>"
custom_edit_url: null
---

[server](../modules/server.md).Expression

Las expresiones se evalúan para producir un [Valor](../modules/values.md#value) en el transcurso de la ejecución de una consulta.

Para construir una expresión, utiliza el [FilterBuilder](../interfaces/server.FilterBuilder.md) proporcionado dentro de
[filter](../interfaces/server.OrderedQuery.md#filter).

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `T` | extends [`Value`](../modules/values.md#value) | `undefined` | Tipo al que se evalúa esta expresión. |