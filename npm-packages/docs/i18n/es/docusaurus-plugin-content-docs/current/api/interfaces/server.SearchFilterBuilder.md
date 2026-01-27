---
id: "server.SearchFilterBuilder"
title: "Interfaz: SearchFilterBuilder<Document, SearchIndexConfig>"
custom_edit_url: null
---

[server](../modules/server.md).SearchFilterBuilder

Constructor para definir filtros de búsqueda.

Un filtro de búsqueda es una lista encadenada de:

1. Una expresión de búsqueda construida con `.search`.
2. Cero o más expresiones de igualdad construidas con `.eq`.

La expresión de búsqueda debe buscar texto en el `searchField` del índice. Las
expresiones de filtro pueden usar cualquiera de los `filterFields` definidos en el índice.

Para cualquier otro tipo de filtrado, usa [filter](server.OrderedQuery.md#filter).

Para obtener más información sobre la búsqueda de texto completo, consulta [Indexes](https://docs.convex.dev/text-search).

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Document` | extiende [`GenericDocument`](../modules/server.md#genericdocument) |
| `SearchIndexConfig` | extiende [`GenericSearchIndexConfig`](../modules/server.md#genericsearchindexconfig) |

## Métodos \{#methods\}

### search \{#search\}

▸ **search**(`fieldName`, `query`): [`SearchFilterFinalizer`](server.SearchFilterFinalizer.md)&lt;`Document`, `SearchIndexConfig`&gt;

Busca los términos de `query` dentro de `doc[fieldName]`.

Realiza una búsqueda de texto completo que devuelve resultados en los que
cualquier palabra de `query` aparece en el campo.

Los documentos se devolverán en función de su relevancia con respecto a la consulta. Esto
tiene en cuenta:

* ¿Cuántas palabras de la consulta aparecen en el texto?
* ¿Cuántas veces aparecen?
* ¿Cuál es la longitud del campo de texto?

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `fieldName` | `SearchIndexConfig`[`"searchField"`] | El nombre del campo en el que buscar. Debe estar configurado como el `searchField` del índice. |
| `query` | `string` | El texto de la consulta de búsqueda. |

#### Devuelve \{#returns\}

[`SearchFilterFinalizer`](server.SearchFilterFinalizer.md)&lt;`Document`, `SearchIndexConfig`&gt;

#### Definido en \{#defined-in\}

[server/search&#95;filter&#95;builder.ts:42](https://github.com/get-convex/convex-js/blob/main/src/server/search_filter_builder.ts#L42)