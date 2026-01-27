---
id: "server.PaginationOptions"
title: "Interfaz: PaginationOptions"
custom_edit_url: null
---

[server](../modules/server.md).PaginationOptions

Las opciones que se le pasan a [paginate](server.OrderedQuery.md#paginate).

Para utilizar este tipo en la [validación de argumentos](https://docs.convex.dev/functions/validation),
usa el [paginationOptsValidator](../modules/server.md#paginationoptsvalidator).

## Propiedades \{#properties\}

### numItems \{#numitems\}

• **numItems**: `number`

Número de elementos que se cargarán en esta página de resultados.

Nota: ¡Este es solo un valor inicial!

Si estás ejecutando esta consulta paginada en una función de consulta reactiva,
podrías recibir más o menos elementos que los indicados aquí si se agregan o
eliminan elementos del rango de la consulta.

#### Definido en \{#defined-in\}

[server/pagination.ts:78](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L78)

***

### cursor \{#cursor\}

• **cursor**: `null` | `string`

Un [Cursor](../modules/server.md#cursor) que representa el inicio de esta página o `null` para comenzar
desde el principio de los resultados de la consulta.

#### Definido en \{#defined-in\}

[server/pagination.ts:84](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L84)