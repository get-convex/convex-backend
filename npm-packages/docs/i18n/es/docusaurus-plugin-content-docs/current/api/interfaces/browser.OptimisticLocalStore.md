---
id: "browser.OptimisticLocalStore"
title: "Interfaz: OptimisticLocalStore"
custom_edit_url: null
---

[browser](../modules/browser.md).OptimisticLocalStore

Una vista de los resultados de las consultas que están actualmente en el cliente de Convex para usar en
actualizaciones optimistas.

## Métodos \{#methods\}

### getQuery \{#getquery\}

▸ **getQuery**&lt;`Query`&gt;(`query`, `...args`): `undefined` | [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;

Recupera el resultado de una consulta del cliente.

Importante: ¡Los resultados de las consultas deben tratarse como inmutables!
Crea siempre nuevas copias de las estructuras dentro de los resultados de consultas para evitar
dañar los datos en el cliente.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `query` | `Query` | Una [FunctionReference](../modules/server.md#functionreference) de la consulta que se va a obtener. |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; | El objeto de argumentos para esta consulta. |

#### Returns \{#returns\}

`undefined` | [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;

El resultado de la consulta o `undefined` si la consulta no se encuentra actualmente
en el cliente.

#### Definido en \{#defined-in\}

[browser/sync/optimistic&#95;updates.ts:28](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/optimistic_updates.ts#L28)

***

### getAllQueries \{#getallqueries\}

▸ **getAllQueries**&lt;`Query`&gt;(`query`): &#123; `args`: [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Query`&gt; ; `value`: `undefined` | [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;  &#125;[]

Obtiene los resultados y los argumentos de todas las consultas con un nombre determinado.

Esto es útil para actualizaciones optimistas complejas que necesitan inspeccionar y
actualizar muchos resultados de consultas (por ejemplo, al actualizar una lista paginada).

Importante: ¡Los resultados de las consultas deben tratarse como inmutables!
Crea siempre nuevas copias de las estructuras dentro de los resultados de las consultas para evitar
corromper los datos en el cliente.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `query` | `Query` | Una [FunctionReference](../modules/server.md#functionreference) para la consulta que se va a obtener. |

#### Devuelve \{#returns\}

&#123; `args`: [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Query`&gt; ; `value`: `undefined` | [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;  &#125;[]

Un array de objetos, uno por cada consulta con ese nombre.
Cada objeto incluye:

* `args` - El objeto de argumentos de la consulta.
  * `value` El resultado de la consulta o `undefined` si la consulta se está cargando.

#### Definido en \{#defined-in\}

[browser/sync/optimistic&#95;updates.ts:49](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/optimistic_updates.ts#L49)

***

### setQuery \{#setquery\}

▸ **setQuery**&lt;`Query`&gt;(`query`, `args`, `value`): `void`

Actualiza de forma optimista el resultado de una consulta.

Puede ser un nuevo valor (quizás derivado del valor anterior obtenido de
[getQuery](browser.OptimisticLocalStore.md#getquery)) o `undefined` para eliminar la consulta.
Eliminar una consulta es útil para crear estados de carga mientras Convex recalcula
los resultados de la consulta.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extiende [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### Parámetros \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | Una [FunctionReference](../modules/server.md#functionreference) de la consulta que se va a establecer. |
| `args` | [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Query`&gt; | El objeto de argumentos para esta consulta. |
| `value` | `undefined` | [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt; | El nuevo valor para establecer la consulta o `undefined` para eliminarla del cliente. |

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[browser/sync/optimistic&#95;updates.ts:69](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/optimistic_updates.ts#L69)