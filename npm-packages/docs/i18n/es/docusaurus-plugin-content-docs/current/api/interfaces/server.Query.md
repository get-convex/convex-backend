---
id: "server.Query"
title: "Interfaz: Query<TableInfo>"
custom_edit_url: null
---

[server](../modules/server.md).Query

La interfaz [Query](server.Query.md) permite que las funciones lean valores de la base de datos.

**Si solo necesitas cargar un objeto por ID, usa `db.get(id)` en su lugar.**

Ejecutar una consulta consiste en llamar a:

1. (Opcional) [order](server.Query.md#order) para definir el orden
2. (Opcional) [filter](server.OrderedQuery.md#filter) para refinar los resultados
3. Un método de *consumo* para obtener los resultados

Las consultas se evalúan de forma diferida. No se realiza ningún trabajo hasta que comienza la iteración, por lo que construir y
extender una consulta no tiene coste. La consulta se ejecuta de forma incremental a medida que se iteran los resultados,
por lo que finalizar antes también reduce el coste de la consulta.

Es más eficiente usar una expresión `filter` que ejecutar JavaScript para filtrar.

|                                              | |
|----------------------------------------------|-|
| **Ordenación**                               | |
| [`order("asc")`](#order)                     | Define el orden de los resultados de la consulta. |
|                                              | |
| **Filtrado**                                 | |
| [`filter(...)`](#filter)                     | Filtra los resultados de la consulta para incluir solo los valores que coinciden con alguna condición. |
|                                              | |
| **Consumo**                                  | Ejecuta una consulta y devuelve los resultados de diferentes maneras. |
| [`[Symbol.asyncIterator]()`](#asynciterator) | Los resultados de la consulta pueden iterarse usando un bucle `for await..of`. |
| [`collect()`](#collect)                      | Devuelve todos los resultados como un array. |
| [`take(n: number)`](#take)                   | Devuelve los primeros `n` resultados como un array. |
| [`first()`](#first)                          | Devuelve el primer resultado. |
| [`unique()`](#unique)                        | Devuelve el único resultado y lanza una excepción si hay más de un resultado. |

Para obtener más información sobre cómo escribir consultas, consulta [Querying the Database](https://docs.convex.dev/using/database-queries).

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableInfo` | extiende [`GenericTableInfo`](../modules/server.md#generictableinfo) |

## Jerarquía \{#hierarchy\}

* [`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

  ↳ **`Query`**

  ↳↳ [`QueryInitializer`](server.QueryInitializer.md)

## Métodos \{#methods\}

### [asyncIterator]

▸ **[asyncIterator]**(): `AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### Devuelve \{#returns\}

`AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### Heredado de \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[[asyncIterator]](server.OrderedQuery.md#[asynciterator])

#### Definido en \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es2018.asynciterable.d.ts:38

***

### order \{#order\}

▸ **order**(`order`): [`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

Define el orden del resultado de la consulta.

Usa `"asc"` para un orden ascendente y `"desc"` para un orden descendente. Si no se especifica, el orden predeterminado es ascendente.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `order` | `"asc"` | `"desc"` | Orden en el que se devolverán los resultados. |

#### Devuelve \{#returns\}

[`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

#### Definido en \{#defined-in\}

[server/query.ts:149](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L149)

***

### filter \{#filter\}

▸ **filter**(`predicate`): [`Query`](server.Query.md)&lt;`TableInfo`&gt;

Filtra la salida de la consulta y devuelve solo los valores para los que `predicate` se evalúa como true.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `predicate` | (`q`: [`FilterBuilder`](server.FilterBuilder.md)&lt;`TableInfo`&gt;) =&gt; [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt; | Una [Expression](../classes/server.Expression.md) construida con el [FilterBuilder](server.FilterBuilder.md) proporcionado que especifica qué documentos deben conservarse. |

#### Devuelve \{#returns\}

[`Query`](server.Query.md)&lt;`TableInfo`&gt;

* Una nueva [OrderedQuery](server.OrderedQuery.md) con el predicado de filtro especificado aplicado.

#### Heredado de \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[filter](server.OrderedQuery.md#filter)

#### Definido en \{#defined-in\}

[server/query.ts:165](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L165)

***

### paginate \{#paginate\}

▸ **paginate**(`paginationOpts`): `Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

Carga una página de `n` resultados y obtiene un [Cursor](../modules/server.md#cursor) para cargar más.

Nota: Si se llama desde una función de consulta reactiva, el número de
resultados puede no coincidir con `paginationOpts.numItems`.

`paginationOpts.numItems` es solo un valor inicial. Después de la primera invocación,
`paginate` devolverá todos los elementos en el rango de la consulta original. Esto garantiza
que todas las páginas permanezcan adyacentes y no se superpongan.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `paginationOpts` | [`PaginationOptions`](server.PaginationOptions.md) | Un objeto [PaginationOptions](server.PaginationOptions.md) que contiene la cantidad de elementos a cargar y el cursor desde el cual empezar. |

#### Returns \{#returns\}

`Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

Un [PaginationResult](server.PaginationResult.md) que contiene la página de resultados y un
cursor para continuar la paginación.

#### Heredado de \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[paginate](server.OrderedQuery.md#paginate)

#### Definido en \{#defined-in\}

[server/query.ts:194](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L194)

***

### collect \{#collect\}

▸ **collect**(): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

Ejecuta la consulta y devuelve todos los resultados en un array.

Nota: al procesar una consulta con muchos resultados, a menudo es mejor usar la `Query` como un
`AsyncIterable`.

#### Devuelve \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* Un array con todos los resultados de la consulta.

#### Heredado de \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[collect](server.OrderedQuery.md#collect)

#### Definido en \{#defined-in\}

[server/query.ts:206](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L206)

***

### take \{#take\}

▸ **take**(`n`): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

Ejecuta la consulta y devuelve los primeros `n` resultados.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `n` | `number` | La cantidad de elementos que se deben obtener. |

#### Devuelve \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* Un array con los primeros `n` resultados de la consulta (o menos, si la
  consulta no tiene `n` resultados).

#### Heredado de \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[take](server.OrderedQuery.md#take)

#### Definido en \{#defined-in\}

[server/query.ts:215](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L215)

***

### first \{#first\}

▸ **first**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

Ejecuta la consulta y devuelve el primer resultado si existe.

#### Returns \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* El primer valor de la consulta o `null` si la consulta no devolvió ningún resultado.

#### Heredado de \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[first](server.OrderedQuery.md#first)

#### Definido en \{#defined-in\}

[server/query.ts:222](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L222)

***

### unique \{#unique\}

▸ **unique**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

Ejecuta la consulta y devuelve un único resultado si existe.

**`Throws`**

Lanzará un error si la consulta devuelve más de un resultado.

#### Returns \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* El único resultado devuelto por la consulta o null si no hay ninguno.

#### Heredado de \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[unique](server.OrderedQuery.md#unique)

#### Definido en \{#defined-in\}

[server/query.ts:230](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L230)