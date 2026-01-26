---
id: "server.OrderedQuery"
title: "Interfaz: OrderedQuery<TableInfo>"
custom_edit_url: null
---

[server](../modules/server.md).OrderedQuery

Una [Query](server.Query.md) con un orden ya definido.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableInfo` | extiende de [`GenericTableInfo`](../modules/server.md#generictableinfo) |

## Jerarquía \{#hierarchy\}

* `AsyncIterable`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

  ↳ **`OrderedQuery`**

  ↳↳ [`Query`](server.Query.md)

## Métodos \{#methods\}

### [asyncIterator]

▸ **[asyncIterator]**(): `AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### Devuelve \{#returns\}

`AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### Heredado de \{#inherited-from\}

AsyncIterable.[asyncIterator]

#### Definido en \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es2018.asynciterable.d.ts:38

***

### filter \{#filter\}

▸ **filter**(`predicate`): [`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

Filtra la salida de la consulta y devuelve solo los valores para los que `predicate` devuelve true.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `predicate` | (`q`: [`FilterBuilder`](server.FilterBuilder.md)&lt;`TableInfo`&gt;) =&gt; [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt; | Una [Expression](../classes/server.Expression.md) construida con el [FilterBuilder](server.FilterBuilder.md) proporcionado que especifica qué documentos se deben conservar. |

#### Devuelve \{#returns\}

[`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

* Un nuevo [OrderedQuery](server.OrderedQuery.md) con el predicado de filtro especificado aplicado.

#### Definido en \{#defined-in\}

[server/query.ts:165](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L165)

***

### paginate \{#paginate\}

▸ **paginate**(`paginationOpts`): `Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

Carga una página de `n` resultados y obtiene un [Cursor](../modules/server.md#cursor) para cargar más.

Nota: Si se llama desde una función de consulta reactiva, el número de
resultados puede no coincidir con `paginationOpts.numItems`.

`paginationOpts.numItems` es solo un valor inicial. Después de la primera llamada,
`paginate` devolverá todos los elementos en el rango original de la consulta. Esto garantiza
que todas las páginas se mantengan adyacentes y sin solaparse.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `paginationOpts` | [`PaginationOptions`](server.PaginationOptions.md) | Un objeto [PaginationOptions](server.PaginationOptions.md) que contiene el número de elementos a cargar y el cursor desde el que iniciar. |

#### Devuelve \{#returns\}

`Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

Un [PaginationResult](server.PaginationResult.md) que contiene la página de resultados y un
cursor para continuar con la paginación.

#### Definido en \{#defined-in\}

[server/query.ts:194](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L194)

***

### collect \{#collect\}

▸ **collect**(): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

Ejecuta la consulta y devuelve todos los resultados en un array.

Nota: al procesar una consulta con muchos resultados, a menudo es mejor usar la `Query` como
`AsyncIterable` en su lugar.

#### Devuelve \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* Una matriz con todos los resultados de la consulta.

#### Definido en \{#defined-in\}

[server/query.ts:206](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L206)

***

### take \{#take\}

▸ **take**(`n`): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

Ejecuta la consulta y devuelve los `n` primeros resultados.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `n` | `number` | El número de elementos que se obtendrán. |

#### Devuelve \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* Una matriz con los primeros `n` resultados de la consulta (o menos si la
  consulta no tiene `n` resultados).

#### Definido en \{#defined-in\}

[server/query.ts:215](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L215)

***

### first \{#first\}

▸ **first**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

Ejecuta la consulta y devuelve el primer resultado si hay alguno.

#### Returns \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* El primer valor de la consulta o `null` si la consulta no devuelve ningún resultado.

#### Definido en \{#defined-in\}

[server/query.ts:222](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L222)

***

### unique \{#unique\}

▸ **unique**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

Ejecuta la consulta y devuelve el resultado único si hay uno.

**`Throws`**

Lanzará un error si la consulta devuelve más de un resultado.

#### Returns \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* El único resultado devuelto por la consulta o `null` si no hay ninguno.

#### Definido en \{#defined-in\}

[server/query.ts:230](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L230)