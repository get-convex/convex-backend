---
id: "server.VectorSearchQuery"
title: "Interface: VectorSearchQuery<TableInfo, IndexName>"
custom_edit_url: null
---

[server](../modules/server.md).VectorSearchQuery

Un objeto con parámetros para realizar una búsqueda vectorial en un índice vectorial.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableInfo` | extiende [`GenericTableInfo`](../modules/server.md#generictableinfo) |
| `IndexName` | extiende [`VectorIndexNames`](../modules/server.md#vectorindexnames)&lt;`TableInfo`&gt; |

## Propiedades \{#properties\}

### vector \{#vector\}

• **vector**: `number`[]

El vector de consulta.

Debe tener la misma longitud que las `dimensions` del índice.
Esta búsqueda vectorial devolverá los Id de los documentos más similares a
este vector.

#### Definido en \{#defined-in\}

[server/vector&#95;search.ts:30](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L30)

***

### limit \{#limit\}

• `Opcional` **limit**: `number`

El número de resultados que se devolverán. Si se especifica, debe estar entre 1 y 256 inclusive.

**`Predeterminado`**

```ts
10
```

#### Definido en \{#defined-in\}

[server/vector&#95;search.ts:37](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L37)

***

### filter \{#filter\}

• `Optional` **filter**: (`q`: [`VectorFilterBuilder`](server.VectorFilterBuilder.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, [`NamedVectorIndex`](../modules/server.md#namedvectorindex)&lt;`TableInfo`, `IndexName`&gt;&gt;) =&gt; [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

#### Declaración de tipo \{#type-declaration\}

▸ (`q`): [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

Expresión de filtro opcional compuesta por `q.or` y `q.eq` que operan
sobre los campos de filtro del índice.

p. ej. `filter: q => q.or(q.eq("genre", "comedy"), q.eq("genre", "drama"))`

##### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `q` | [`VectorFilterBuilder`](server.VectorFilterBuilder.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, [`NamedVectorIndex`](../modules/server.md#namedvectorindex)&lt;`TableInfo`, `IndexName`&gt;&gt; |

##### Devuelve \{#returns\}

[`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

#### Definido en \{#defined-in\}

[server/vector&#95;search.ts:47](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L47)