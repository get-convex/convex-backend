---
id: "server.VectorFilterBuilder"
title: "Interfaz: VectorFilterBuilder<Document, VectorIndexConfig>"
custom_edit_url: null
---

[server](../modules/server.md).VectorFilterBuilder

Interfaz para definir filtros para búsquedas vectoriales.

Esta interfaz tiene una API similar a [FilterBuilder](server.FilterBuilder.md), que se usa en
consultas a la base de datos, pero solo admite los métodos que se pueden ejecutar de manera eficiente
en una búsqueda vectorial.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Document` | extiende de [`GenericDocument`](../modules/server.md#genericdocument) |
| `VectorIndexConfig` | extiende de [`GenericVectorIndexConfig`](../modules/server.md#genericvectorindexconfig) |

## Métodos \{#methods\}

### eq \{#eq\}

▸ **eq**&lt;`FieldName`&gt;(`fieldName`, `value`): [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

Indica si el campo en `fieldName` es igual a `value`

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `FieldName` | extends `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `fieldName` | `FieldName` |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `FieldName`&gt; |

#### Devuelve \{#returns\}

[`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

#### Definido en \{#defined-in\}

[server/vector&#95;search.ts:110](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L110)

***

### or \{#or\}

▸ **or**(`...exprs`): [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

`exprs[0] || exprs[1] || ... || exprs[n]`

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `...exprs` | [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;[] |

#### Devuelve \{#returns\}

[`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt;

#### Definido en \{#defined-in\}

[server/vector&#95;search.ts:122](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L122)