---
id: "server.IndexRangeBuilder"
title: "Interfaz: IndexRangeBuilder<Document, IndexFields, FieldNum>"
custom_edit_url: null
---

[server](../modules/server.md).IndexRangeBuilder

Constructor para definir un rango de índice para realizar una consulta.

Un rango de índice es una descripción de qué documentos debe considerar Convex
al ejecutar la consulta.

Un rango de índice es siempre una lista encadenada de:

1. 0 o más expresiones de igualdad definidas con `.eq`.
2. [Opcionalmente] Una expresión de cota inferior definida con `.gt` o `.gte`.
3. [Opcionalmente] Una expresión de cota superior definida con `.lt` o `.lte`.

**Debes avanzar por los campos en el orden del índice.**

Cada expresión de igualdad debe comparar un campo de índice diferente, empezando desde
el principio y en orden. Las cotas superior e inferior deben seguir a las
expresiones de igualdad y comparar el campo siguiente.

Por ejemplo, si hay un índice de mensajes en
`["projectId", "priority"]`, un rango que busque &quot;mensajes en &#39;myProjectId&#39;
con prioridad de al menos 100&quot; se vería así:

```ts
q.eq("projectId", myProjectId)
 .gte("priority", 100)
```

**El rendimiento de tu consulta depende de la especificidad del rango.**

Esta clase está diseñada para permitirte especificar únicamente rangos que Convex pueda aprovechar de forma eficiente con tu índice para encontrar resultados. Para cualquier otro tipo de filtrado, usa
[filter](server.OrderedQuery.md#filter).

Para aprender más sobre índices, consulta [Indexes](https://docs.convex.dev/using/indexes).

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Document` | extiende [`GenericDocument`](../modules/server.md#genericdocument) |
| `IndexFields` | extiende [`GenericIndexFields`](../modules/server.md#genericindexfields) |
| `FieldNum` | extiende `number` = `0` |

## Jerarquía \{#hierarchy\}

* `LowerBoundIndexRangeBuilder`&lt;`Document`, `IndexFields`[`FieldNum`]&gt;

  ↳ **`IndexRangeBuilder`**

## Métodos \{#methods\}

### eq \{#eq\}

▸ **eq**(`fieldName`, `value`): `NextIndexRangeBuilder`&lt;`Document`, `IndexFields`, `FieldNum`&gt;

Limita este rango a los documentos en los que `doc[fieldName] === value`.

#### Parámetros \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `fieldName` | `IndexFields`[`FieldNum`] | El nombre del campo que se va a comparar. Debe ser el siguiente campo en el índice. |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `IndexFields`[`FieldNum`]&gt; | El valor contra el que se va a comparar. |

#### Devuelve \{#returns\}

`NextIndexRangeBuilder`&lt;`Document`, `IndexFields`, `FieldNum`&gt;

#### Definido en \{#defined-in\}

[server/index&#95;range&#95;builder.ts:76](https://github.com/get-convex/convex-js/blob/main/src/server/index_range_builder.ts#L76)

***

### gt \{#gt\}

▸ **gt**(`fieldName`, `value`): `UpperBoundIndexRangeBuilder`&lt;`Document`, `IndexFields`[`FieldNum`]&gt;

Restringe este rango a los documentos en los que `doc[fieldName] > value`.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `fieldName` | `IndexFields`[`FieldNum`] | El nombre del campo que se va a comparar. Debe ser el siguiente campo en el índice. |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `IndexFields`[`FieldNum`]&gt; | El valor con el que se va a comparar. |

#### Devuelve \{#returns\}

`UpperBoundIndexRangeBuilder`&lt;`Document`, `IndexFields`[`FieldNum`]&gt;

#### Heredado de \{#inherited-from\}

LowerBoundIndexRangeBuilder.gt

#### Definido en \{#defined-in\}

[server/index&#95;range&#95;builder.ts:115](https://github.com/get-convex/convex-js/blob/main/src/server/index_range_builder.ts#L115)

***

### gte \{#gte\}

▸ **gte**(`fieldName`, `value`): `UpperBoundIndexRangeBuilder`&lt;`Document`, `IndexFields`[`FieldNum`]&gt;

Restringe este rango a los documentos en los que `doc[fieldName] >= value`.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `fieldName` | `IndexFields`[`FieldNum`] | El nombre del campo que se va a comparar. Debe ser el siguiente campo en el índice. |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `IndexFields`[`FieldNum`]&gt; | El valor con el que se va a comparar. |

#### Devuelve \{#returns\}

`UpperBoundIndexRangeBuilder`&lt;`Document`, `IndexFields`[`FieldNum`]&gt;

#### Heredado de \{#inherited-from\}

LowerBoundIndexRangeBuilder.gte

#### Definido en \{#defined-in\}

[server/index&#95;range&#95;builder.ts:126](https://github.com/get-convex/convex-js/blob/main/src/server/index_range_builder.ts#L126)

***

### lt \{#lt\}

▸ **lt**(`fieldName`, `value`): [`IndexRange`](../classes/server.IndexRange.md)

Limita este rango a los documentos en los que `doc[fieldName] < value`.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `fieldName` | `IndexFields`[`FieldNum`] | Nombre del campo que se va a comparar. Debe ser el mismo campo de índice usado en el límite inferior (`.gt` o `.gte`) o el siguiente campo si no se especificó un límite inferior. |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `IndexFields`[`FieldNum`]&gt; | El valor con el que se va a comparar. |

#### Devuelve \{#returns\}

[`IndexRange`](../classes/server.IndexRange.md)

#### Heredado de \{#inherited-from\}

LowerBoundIndexRangeBuilder.lt

#### Definido en \{#defined-in\}

[server/index&#95;range&#95;builder.ts:151](https://github.com/get-convex/convex-js/blob/main/src/server/index_range_builder.ts#L151)

***

### lte \{#lte\}

▸ **lte**(`fieldName`, `value`): [`IndexRange`](../classes/server.IndexRange.md)

Restringe este rango a documentos en los que `doc[fieldName] <= value`.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `fieldName` | `IndexFields`[`FieldNum`] | El nombre del campo que se va a comparar. Debe ser el mismo campo de índice usado en el límite inferior (`.gt` o `.gte`) o el siguiente campo si no se especificó ningún límite inferior. |
| `value` | [`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;`Document`, `IndexFields`[`FieldNum`]&gt; | El valor con el que se va a comparar. |

#### Devuelve \{#returns\}

[`IndexRange`](../classes/server.IndexRange.md)

#### Heredado de \{#inherited-from\}

LowerBoundIndexRangeBuilder.lte

#### Definido en \{#defined-in\}

[server/index&#95;range&#95;builder.ts:164](https://github.com/get-convex/convex-js/blob/main/src/server/index_range_builder.ts#L164)