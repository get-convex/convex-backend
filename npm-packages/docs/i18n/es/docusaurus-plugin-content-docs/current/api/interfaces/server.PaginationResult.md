---
id: "server.PaginationResult"
title: "Interfaz: PaginationResult<T>"
custom_edit_url: null
---

[server](../modules/server.md).PaginationResult

El resultado de la paginación con [paginate](server.OrderedQuery.md#paginate).

## Parámetros de tipo \{#type-parameters\}

| Nombre |
| :------ |
| `T` |

## Propiedades \{#properties\}

### page \{#page\}

• **page**: `T`[]

La página de resultados.

#### Definido en \{#defined-in\}

[server/pagination.ts:32](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L32)

***

### isDone \{#isdone\}

• **isDone**: `boolean`

¿Hemos llegado al final de los resultados?

#### Definido en \{#defined-in\}

[server/pagination.ts:37](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L37)

***

### continueCursor \{#continuecursor\}

• **continueCursor**: `string`

Un [Cursor](../modules/server.md#cursor) para continuar cargando más resultados.

#### Definido en \{#defined-in\}

[server/pagination.ts:42](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L42)

***

### splitCursor \{#splitcursor\}

• `Optional` **splitCursor**: `null` | `string`

Un [Cursor](../modules/server.md#cursor) para dividir la página en dos, de modo que la página desde
(cursor, continueCursor] pueda reemplazarse por dos páginas: (cursor, splitCursor]
y (splitCursor, continueCursor].

#### Definido en \{#defined-in\}

[server/pagination.ts:49](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L49)

***

### pageStatus \{#pagestatus\}

• `Optional` **pageStatus**: `null` | `"SplitRecommended"` | `"SplitRequired"`

Cuando una consulta lee demasiados datos, puede devolver &#39;SplitRecommended&#39; para
indicar que la página debe dividirse en dos con `splitCursor`.
Cuando una consulta lee tantos datos que `page` podría estar incompleta, su status
pasa a ser &#39;SplitRequired&#39;.

#### Definido en \{#defined-in\}

[server/pagination.ts:57](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L57)