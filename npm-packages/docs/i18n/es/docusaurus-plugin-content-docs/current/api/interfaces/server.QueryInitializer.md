---
id: "server.QueryInitializer"
title: "Interfaz: QueryInitializer<TableInfo>"
custom_edit_url: null
---

[server](../modules/server.md).QueryInitializer

La interfaz [QueryInitializer](server.QueryInitializer.md) es el punto de entrada para construir una [Query](server.Query.md)
sobre una tabla de la base de datos de Convex.

Hay dos tipos de consultas:

1. Recorridos completos de tabla: consultas creadas con [fullTableScan](server.QueryInitializer.md#fulltablescan) que
   iteran sobre todos los documentos de la tabla en orden de inserción.
2. Consultas indexadas: consultas creadas con [withIndex](server.QueryInitializer.md#withindex) que iteran
   sobre un rango de índice en orden de índice.

Por comodidad, [QueryInitializer](server.QueryInitializer.md) extiende la interfaz [Query](server.Query.md), iniciando implícitamente
un recorrido completo de la tabla.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableInfo` | extiende de [`GenericTableInfo`](../modules/server.md#generictableinfo) |

## Jerarquía \{#hierarchy\}

* [`Query`](server.Query.md)&lt;`TableInfo`&gt;

  ↳ **`QueryInitializer`**

## Métodos \{#methods\}

### [asyncIterator]

▸ **[asyncIterator]**(): `AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### Devuelve \{#returns\}

`AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### Heredado de \{#inherited-from\}

[Query](server.Query.md).[[asyncIterator]](server.Query.md#[asynciterator])

#### Definido en \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es2018.asynciterable.d.ts:38

***

### fullTableScan \{#fulltablescan\}

▸ **fullTableScan**(): [`Query`](server.Query.md)&lt;`TableInfo`&gt;

Consulta que lee todos los valores de esta tabla.

El costo de esta consulta es relativo al tamaño de toda la tabla, por lo que solo debería usarse en tablas que permanecerán muy pequeñas (digamos entre unos pocos cientos y unos pocos miles de documentos) y que se actualizan con poca frecuencia.

#### Devuelve \{#returns\}

[`Query`](server.Query.md)&lt;`TableInfo`&gt;

* La [Query](server.Query.md) que recorre cada documento de la tabla.

#### Definido en \{#defined-in\}

[server/query.ts:40](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L40)

***

### withIndex \{#withindex\}

▸ **withIndex**&lt;`IndexName`&gt;(`indexName`, `indexRange?`): [`Query`](server.Query.md)&lt;`TableInfo`&gt;

Consulta mediante la lectura de documentos desde un índice de esta tabla.

El costo de esta consulta es proporcional al número de documentos que cumplen
la expresión de rango del índice.

Los resultados se devolverán en el orden del índice.

Para obtener más información sobre los índices, consulta [Indexes](https://docs.convex.dev/using/indexes).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `IndexName` | extends `string` | `number` | `symbol` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `indexName` | `IndexName` | El nombre del índice que se va a consultar. |
| `indexRange?` | (`q`: [`IndexRangeBuilder`](server.IndexRangeBuilder.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, [`NamedIndex`](../modules/server.md#namedindex)&lt;`TableInfo`, `IndexName`&gt;, `0`&gt;) =&gt; [`IndexRange`](../classes/server.IndexRange.md) | Un rango de índice opcional construido mediante el [IndexRangeBuilder](server.IndexRangeBuilder.md) proporcionado. Un rango de índice describe qué documentos debe tener en cuenta Convex al ejecutar la consulta. Si no se proporciona un rango de índice, la consulta considerará todos los documentos del índice. |

#### Devuelve \{#returns\}

[`Query`](server.Query.md)&lt;`TableInfo`&gt;

* La consulta que devuelve documentos del índice.

#### Definido en \{#defined-in\}

[server/query.ts:59](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L59)

***

### withSearchIndex \{#withsearchindex\}

▸ **withSearchIndex**&lt;`IndexName`&gt;(`indexName`, `searchFilter`): [`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

Realiza una consulta mediante una búsqueda de texto completo en un índice de búsqueda.

Las consultas de búsqueda siempre deben buscar algún texto dentro del
`searchField` del índice. Esta consulta puede opcionalmente agregar filtros de igualdad para cualesquiera
`filterFields` especificados en el índice.

Los documentos se devolverán en orden de relevancia según qué tan bien
coincidan con el texto de búsqueda.

Para obtener más información sobre la búsqueda de texto completo, consulta [Indexes](https://docs.convex.dev/text-search).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `IndexName` | extends `string` | `number` | `symbol` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `indexName` | `IndexName` | El nombre del índice de búsqueda en el que se realizará la consulta. |
| `searchFilter` | (`q`: [`SearchFilterBuilder`](server.SearchFilterBuilder.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, [`NamedSearchIndex`](../modules/server.md#namedsearchindex)&lt;`TableInfo`, `IndexName`&gt;&gt;) =&gt; [`SearchFilter`](../classes/server.SearchFilter.md) | Una expresión de filtro de búsqueda creada con el [SearchFilterBuilder](server.SearchFilterBuilder.md) proporcionado. Define la búsqueda de texto completo que se ejecutará junto con el filtrado por igualdad que se aplicará dentro del índice de búsqueda. |

#### Returns \{#returns\}

[`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

* Una consulta que busca documentos que coincidan y los devuelve
  en orden de relevancia.

#### Definido en \{#defined-in\}

[server/query.ts:88](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L88)

***

### order \{#order\}

▸ **order**(`order`): [`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

Define el orden del resultado de la consulta.

Usa `"asc"` para un orden ascendente y `"desc"` para un orden descendente. Si no se especifica, de forma predeterminada el orden es ascendente.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `order` | `"asc"` | `"desc"` | Orden en el que se devolverán los resultados. |

#### Devuelve \{#returns\}

[`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

#### Heredado de \{#inherited-from\}

[Query](server.Query.md).[order](server.Query.md#order)

#### Definido en \{#defined-in\}

[server/query.ts:149](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L149)

***

### filter \{#filter\}

▸ **filter**(`predicate`): [`QueryInitializer`](server.QueryInitializer.md)&lt;`TableInfo`&gt;

Filtra el resultado de la consulta y devuelve solo los valores para los cuales `predicate` se evalúa como verdadero.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `predicate` | (`q`: [`FilterBuilder`](server.FilterBuilder.md)&lt;`TableInfo`&gt;) =&gt; [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt; | Una [Expression](../classes/server.Expression.md) construida con el [FilterBuilder](server.FilterBuilder.md) proporcionado que determina qué documentos se conservan. |

#### Devuelve \{#returns\}

[`QueryInitializer`](server.QueryInitializer.md)&lt;`TableInfo`&gt;

* Una nueva [OrderedQuery](server.OrderedQuery.md) con el predicado de filtro especificado aplicado.

#### Heredado de \{#inherited-from\}

[Query](server.Query.md).[filter](server.Query.md#filter)

#### Definido en \{#defined-in\}

[server/query.ts:165](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L165)

***

### paginate \{#paginate\}

▸ **paginate**(`paginationOpts`): `Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

Carga una página de `n` resultados y obtiene un [Cursor](../modules/server.md#cursor) para cargar más.

Nota: Si se llama desde una función de consulta reactiva, el número de
resultados puede no coincidir con `paginationOpts.numItems`.

`paginationOpts.numItems` es solo un valor inicial. Después de la primera invocación,
`paginate` devolverá todos los elementos en el rango original de la consulta. Esto garantiza
que todas las páginas seguirán siendo adyacentes y no se solaparán.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `paginationOpts` | [`PaginationOptions`](server.PaginationOptions.md) | Un objeto [PaginationOptions](server.PaginationOptions.md) que contiene la cantidad de elementos a cargar y el cursor desde el cual comenzar. |

#### Devuelve \{#returns\}

`Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

Un [PaginationResult](server.PaginationResult.md) que contiene la página de resultados y un
cursor para continuar la paginación.

#### Heredado de \{#inherited-from\}

[Query](server.Query.md).[paginate](server.Query.md#paginate)

#### Definido en \{#defined-in\}

[server/query.ts:194](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L194)

***

### collect \{#collect\}

▸ **collect**(): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

Ejecuta la consulta y devuelve todos los resultados como un array.

Nota: cuando se procesa una consulta con muchos resultados, suele ser mejor usar la `Query` como
`AsyncIterable` en su lugar.

#### Returns \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* Un array con todos los resultados de la consulta.

#### Heredado de \{#inherited-from\}

[Query](server.Query.md).[collect](server.Query.md#collect)

#### Definido en \{#defined-in\}

[server/query.ts:206](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L206)

***

### take \{#take\}

▸ **take**(`n`): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

Ejecuta la consulta y devuelve los primeros `n` resultados.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `n` | `number` | La cantidad de elementos que se van a tomar. |

#### Devuelve \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* Un array con los primeros `n` resultados de la consulta (o menos si la
  consulta no devuelve `n` resultados).

#### Heredado de \{#inherited-from\}

[Query](server.Query.md).[take](server.Query.md#take)

#### Definido en \{#defined-in\}

[server/query.ts:215](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L215)

***

### first \{#first\}

▸ **first**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

Ejecuta la consulta y devuelve el primer resultado si lo hay.

#### Devuelve \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* El primer valor de la consulta o `null` si la consulta no devuelve ningún resultado.

#### Heredado de \{#inherited-from\}

[Query](server.Query.md).[first](server.Query.md#first)

#### Definido en \{#defined-in\}

[server/query.ts:222](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L222)

***

### unique \{#unique\}

▸ **unique**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

Ejecuta la consulta y devuelve un único resultado si hay uno.

**`Throws`**

Genera un error si la consulta devuelve más de un resultado.

#### Returns \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* El único resultado devuelto por la consulta o null si no hay ninguno.

#### Heredado de \{#inherited-from\}

[Query](server.Query.md).[unique](server.Query.md#unique)

#### Definido en \{#defined-in\}

[server/query.ts:230](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L230)