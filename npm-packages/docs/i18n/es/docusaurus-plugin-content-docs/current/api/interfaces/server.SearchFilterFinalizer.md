---
id: "server.SearchFilterFinalizer"
title: "Interfaz: SearchFilterFinalizer<Document, SearchIndexConfig>"
custom_edit_url: null
---

[server](../modules/server.md).SearchFilterFinalizer

Generador para definir expresiones de igualdad como parte de un filtro de búsqueda.

Consulta [SearchFilterBuilder](server.SearchFilterBuilder.md).

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Document` | extiende [`GenericDocument`](../modules/server.md#genericdocument) |
| `SearchIndexConfig` | extiende [`GenericSearchIndexConfig`](../modules/server.md#genericsearchindexconfig) |

## Jerarquía \{#hierarchy\}

* [`SearchFilter`](../classes/server.SearchFilter.md)

  ↳ **`SearchFilterFinalizer`**

## Métodos \{#methods\}

### eq \{#eq\}

▸ **eq**&lt;`FieldName`&gt;(`fieldName`, `value`): [`SearchFilterFinalizer`](server.SearchFilterFinalizer.md)&lt;`Document`, `SearchIndexConfig`&gt;

Restringe esta consulta a los documentos en los que `doc[fieldName] === value`.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `FieldName` | extends `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `fieldName` | `FieldName` | El nombre del campo que se va a comparar. Debe aparecer en `filterFields` del índice de búsqueda. |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `FieldName`&gt; | El valor con el que se va a comparar. |

#### Devuelve \{#returns\}

[`SearchFilterFinalizer`](server.SearchFilterFinalizer.md)&lt;`Document`, `SearchIndexConfig`&gt;

#### Definido en \{#defined-in\}

[server/search&#95;filter&#95;builder.ts:66](https://github.com/get-convex/convex-js/blob/main/src/server/search_filter_builder.ts#L66)