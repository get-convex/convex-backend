---
id: "values.VInt64"
title: "Clase: VInt64<Type, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VInt64

El tipo del validador `v.int64()`.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `bigint` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## Jerarquía \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VInt64`**

## Constructores \{#constructors\}

### constructor \{#constructor\}

• **new VInt64**&lt;`Type`, `IsOptional`&gt;(`«destructured»`)

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `bigint` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |

#### Heredado de \{#inherited-from\}

BaseValidator&lt;Type, IsOptional&gt;.constructor

#### Definido en \{#defined-in\}

[values/validators.ts:54](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L54)

## Propiedades \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

Solo para TypeScript: el tipo de TS de los valores de JS que valida este validador.

#### Heredado de \{#inherited-from\}

BaseValidator.type

#### Definido en \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

Solo para TypeScript: si se trata de un validador `Object`,
este es el tipo de TypeScript de los nombres de sus propiedades.

#### Heredado de \{#inherited-from\}

BaseValidator.fieldPaths

#### Definido en \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

Indica si este es un validador del valor de una propiedad opcional de un objeto.

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

### kind \{#kind\}

• `Readonly` **kind**: `"int64"`

El tipo de validador, `"int64"`.

#### Definido en \{#defined-in\}

[values/validators.ts:145](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L145)