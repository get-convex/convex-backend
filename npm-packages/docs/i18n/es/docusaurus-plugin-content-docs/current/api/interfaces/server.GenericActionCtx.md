---
id: "server.GenericActionCtx"
title: "Interfaz: GenericActionCtx<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericActionCtx

Un conjunto de servicios para usar dentro de funciones de tipo acción de Convex.

El contexto se pasa como primer argumento de cualquier acción de Convex
que se ejecute en el servidor.

Si estás usando generación de código, usa el tipo `ActionCtx` en
`convex/_generated/server.d.ts`, el cual está tipado para tu modelo de datos.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extiende [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## Propiedades \{#properties\}

### scheduler \{#scheduler\}

• **scheduler**: [`Scheduler`](server.Scheduler.md)

Una utilidad para programar la ejecución de funciones de Convex en el futuro.

#### Definido en \{#defined-in\}

[server/registration.ts:236](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L236)

***

### auth \{#auth\}

• **auth**: [`Auth`](server.Auth.md)

Información sobre el usuario que está autenticado actualmente.

#### Definido en \{#defined-in\}

[server/registration.ts:241](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L241)

***

### storage \{#storage\}

• **storage**: [`StorageActionWriter`](server.StorageActionWriter.md)

Herramienta para leer y escribir archivos en el almacenamiento.

#### Definido en \{#defined-in\}

[server/registration.ts:246](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L246)

## Métodos \{#methods\}

### runQuery \{#runquery\}

▸ **runQuery**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

Ejecuta la consulta de Convex con el nombre y los argumentos indicados.

Considera usar un internalQuery para evitar que los usuarios invoquen la
consulta directamente.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`, `"public"` | `"internal"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `query` | `Query` | Una [FunctionReference](../modules/server.md#functionreference) de la consulta que se va a ejecutar. |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; | Los argumentos de la función de consulta. |

#### Devuelve \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

Una promesa que se resuelve con el resultado de la consulta.

#### Definido en \{#defined-in\}

[server/registration.ts:196](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L196)

***

### runMutation \{#runmutation\}

▸ **runMutation**&lt;`Mutation`&gt;(`mutation`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

Ejecuta la mutación de Convex con el nombre y los argumentos indicados.

Considera usar una internalMutation para evitar que los usuarios invoquen
la mutación directamente.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`, `"public"` | `"internal"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | Una [FunctionReference](../modules/server.md#functionreference) de la mutación que se va a ejecutar. |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Mutation`&gt; | Los argumentos para la función de mutación. |

#### Valor de retorno \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

Una promesa del resultado de la mutación.

#### Definido en \{#defined-in\}

[server/registration.ts:211](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L211)

***

### runAction \{#runaction\}

▸ **runAction**&lt;`Action`&gt;(`action`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

Ejecuta la acción de Convex con el nombre y los argumentos proporcionados.

Considera usar internalAction para evitar que los usuarios invoquen la
acción directamente.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"action"`, `"public"` | `"internal"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `action` | `Action` | Una [FunctionReference](../modules/server.md#functionreference) para ejecutar la acción. |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Action`&gt; | Los argumentos de la función de acción. |

#### Returns \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

Una promesa con el resultado de la acción.

#### Definido en \{#defined-in\}

[server/registration.ts:228](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L228)

***

### vectorSearch \{#vectorsearch\}

▸ **vectorSearch**&lt;`TableName`, `IndexName`&gt;(`tableName`, `indexName`, `query`): `Promise`&lt;&#123; `_id`: [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; ; `_score`: `number`  &#125;[]&gt;

Ejecuta una búsqueda vectorial en la tabla e índice indicados.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableName` | extiende `string` |
| `IndexName` | extiende `string` | `number` | `symbol` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `tableName` | `TableName` | El nombre de la tabla a consultar. |
| `indexName` | `IndexName` | El nombre del índice vectorial de la tabla a consultar. |
| `query` | `Object` | Un [VectorSearchQuery](server.VectorSearchQuery.md) que contiene el vector a consultar, el número de resultados a devolver y los filtros. |
| `query.vector` | `number`[] | El vector de consulta. Debe tener la misma longitud que las `dimensions` del índice. Esta búsqueda vectorial devolverá los ID de los documentos más similares a este vector. |
| `query.limit?` | `number` | El número de resultados a devolver. Si se especifica, debe estar entre 1 y 256 inclusive. **`Default`** `ts 10 ` |
| `query.filter?` | (`q`: [`VectorFilterBuilder`](server.VectorFilterBuilder.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;, [`NamedVectorIndex`](../modules/server.md#namedvectorindex)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;, `IndexName`&gt;&gt;) =&gt; [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt; | Expresión de filtro opcional compuesta por `q.or` y `q.eq` que operan sobre los campos de filtro del índice. p. ej. `filter: q => q.or(q.eq("genre", "comedy"), q.eq("genre", "drama"))` |

#### Devuelve \{#returns\}

`Promise`&lt;&#123; `_id`: [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; ; `_score`: `number`  &#125;[]&gt;

Una promesa de ID y puntuaciones de los documentos con los vectores
más cercanos

#### Definido en \{#defined-in\}

[server/registration.ts:258](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L258)