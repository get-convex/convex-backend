---
id: "values.VLiteral"
title: "Clase: VLiteral<Type, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VLiteral

El tipo del validador `v.literal()`.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `Type` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## Jerarquía \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VLiteral`**

## Constructores \{#constructors\}

### constructor \{#constructor\}

• **new VLiteral**&lt;`Type`, `IsOptional`&gt;(`«destructured»`)

Normalmente deberías usar `v.literal(value)` en su lugar.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `Type` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |
| › `value` | `Type` |

#### Sobrescribe \{#overrides\}

BaseValidator&lt;Type, IsOptional&gt;.constructor

#### Definido en \{#defined-in\}

[values/validators.ts:441](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L441)

## Propiedades \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

Solo para TypeScript, el tipo de TS de los valores de JS validados
por este validador.

#### Heredado de \{#inherited-from\}

BaseValidator.type

#### Definido en \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

Solo para TypeScript, si se trata de un validador `Object`, entonces
este es el tipo de TypeScript de los nombres de sus propiedades.

#### Heredado de \{#inherited-from\}

BaseValidator.fieldPaths

#### Definido en \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

Indica si se trata de un validador del valor de una propiedad de objeto opcional.

#### Heredado de \{#inherited-from\}

BaseValidator.isOptional

#### Definido en \{#defined-in\}

[values/validators.ts:47](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L47)

***

### isConvexValidator \{#isconvexvalidator\}

• `Readonly` **isConvexValidator**: `true`

Siempre `"true"`.

#### Heredado de \{#inherited-from\}

BaseValidator.isConvexValidator

#### Definido en \{#defined-in\}

[values/validators.ts:52](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L52)

***

### value \{#value\}

• `Readonly` **value**: `Type`

El valor al que deben ser iguales los valores validados.

#### Definido en \{#defined-in\}

[values/validators.ts:431](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L431)

***

### kind \{#kind\}

• `Readonly` **kind**: `"literal"`

El tipo de validador, `"literal"`.

#### Definido en \{#defined-in\}

[values/validators.ts:436](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L436)