---
id: "server.FilterBuilder"
title: "Interfaz: FilterBuilder<TableInfo>"
custom_edit_url: null
---

[server](../modules/server.md).FilterBuilder

Una interfaz para definir filtros en consultas.

`FilterBuilder` tiene varios métodos que generan [Expression](../classes/server.Expression.md).
Estas expresiones se pueden anidar junto con constantes para expresar
un predicado de filtro.

`FilterBuilder` se usa dentro de [filter](server.OrderedQuery.md#filter) para crear filtros
de consulta.

Estos son los métodos disponibles:

|                               |                                                   |
|-------------------------------|---------------------------------------------------|
| **Comparaciones**             | Produce un error cuando `l` y `r` no son del mismo tipo. |
| [`eq(l, r)`](#eq)             | `l === r`                                         |
| [`neq(l, r)`](#neq)           | `l !== r`                                         |
| [`lt(l, r)`](#lt)             | `l < r`                                           |
| [`lte(l, r)`](#lte)           | `l <= r`                                          |
| [`gt(l, r)`](#gt)             | `l > r`                                           |
| [`gte(l, r)`](#gte)           | `l >= r`                                          |
|                               |                                                   |
| **Aritmética**                | Produce un error cuando `l` y `r` no son del mismo tipo. |
| [`add(l, r)`](#add)           | `l + r`                                           |
| [`sub(l, r)`](#sub)           | `l - r`                                           |
| [`mul(l, r)`](#mul)           | `l * r`                                           |
| [`div(l, r)`](#div)           | `l / r`                                           |
| [`mod(l, r)`](#mod)           | `l % r`                                           |
| [`neg(x)`](#neg)              | `-x`                                              |
|                               |                                                   |
| **Lógica**                    | Produce un error si algún parámetro no es un `bool`. |
| [`not(x)`](#not)              | `!x`                                              |
| [`and(a, b, ..., z)`](#and)   | `a && b && ... && z`                              |
| [`or(a, b, ..., z)`](#or)     | <code>a &#124;&#124; b &#124;&#124; ... &#124;&#124; z</code> |
|                               |                                                   |
| **Otros**                     |                                                   |
| [`field(fieldPath)`](#field)  | Se evalúa al campo en `fieldPath`.               |

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableInfo` | extiende [`GenericTableInfo`](../modules/server.md#generictableinfo) |

## Métodos \{#methods\}

### eq \{#eq\}

▸ **eq**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l === r`

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extends `undefined` | [`Valor`](../modules/values.md#value) |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### Devuelve \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:87](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L87)

***

### neq \{#neq\}

▸ **neq**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l !== r`

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extends `undefined` | [`Valor`](../modules/values.md#value) |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### Devuelve \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:97](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L97)

***

### lt \{#lt\}

▸ **lt**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l < r`

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extiende [`Valor`](../modules/values.md#value) |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### Devuelve \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:107](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L107)

***

### lte \{#lte\}

▸ **lte**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l <= r`

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extiende [`Value`](../modules/values.md#value) |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### Devuelve \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:117](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L117)

***

### gt \{#gt\}

▸ **gt**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l > r`

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extends [`Value`](../modules/values.md#value) |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### Devuelve \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:127](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L127)

***

### gte \{#gte\}

▸ **gte**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`l >= r`

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extends [`Value`](../modules/values.md#value) |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### Devuelve \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:137](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L137)

***

### add \{#add\}

▸ **add**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`l + r`

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extends [`NumericValue`](../modules/values.md#numericvalue) |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### Devuelve \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:149](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L149)

***

### sub \{#sub\}

▸ **sub**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`l - r`

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extends [`NumericValue`](../modules/values.md#numericvalue) |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### Devuelve \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:159](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L159)

***

### mul \{#mul\}

▸ **mul**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`l * r`

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extiende [`NumericValue`](../modules/values.md#numericvalue) |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### Devuelve \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:169](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L169)

***

### div \{#div\}

▸ **div**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`l / r`

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extiende [`NumericValue`](../modules/values.md#numericvalue) |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### Devuelve \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:179](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L179)

***

### mod \{#mod\}

▸ **mod**&lt;`T`&gt;(`l`, `r`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`l % r`

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extiende de [`NumericValue`](../modules/values.md#numericvalue) |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `l` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |
| `r` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### Devuelve \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:189](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L189)

***

### neg \{#neg\}

▸ **neg**&lt;`T`&gt;(`x`): [`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

`-x`

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extiende [`NumericValue`](../modules/values.md#numericvalue) |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `x` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`T`&gt; |

#### Devuelve \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`T`&gt;

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:199](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L199)

***

### and \{#and\}

▸ **and**(`...exprs`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`exprs[0] && exprs[1] && ... && exprs[n]`

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `...exprs` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt;[] |

#### Devuelve \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:208](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L208)

***

### or \{#or\}

▸ **or**(`...exprs`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`exprs[0] || exprs[1] || ... || exprs[n]`

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `...exprs` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt;[] |

#### Devuelve \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:215](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L215)

***

### not \{#not\}

▸ **not**(`x`): [`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

`!x`

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `x` | [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt; |

#### Devuelve \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;`boolean`&gt;

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:222](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L222)

***

### field \{#field\}

▸ **field**&lt;`FieldPath`&gt;(`fieldPath`): [`Expression`](../classes/server.Expression.md)&lt;[`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `FieldPath`&gt;&gt;

Se evalúa al campo ubicado en la `fieldPath` proporcionada.

Por ejemplo, en [filter](server.OrderedQuery.md#filter) esto se puede usar para examinar los valores que se están filtrando.

#### Ejemplo \{#example\}

En este objeto:

```
{
  "user": {
    "isActive": true
  }
}
```

`field("user.isActive")` se evalúa en `true`.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `FieldPath` | extends `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `fieldPath` | `FieldPath` |

#### Devuelve \{#returns\}

[`Expression`](../classes/server.Expression.md)&lt;[`FieldTypeFromFieldPath`](../modules/server.md#fieldtypefromfieldpath)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `FieldPath`&gt;&gt;

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:246](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L246)