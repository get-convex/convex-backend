---
id: "values.VFloat64"
title: "Clase: VFloat64<Type, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VFloat64

El tipo de validador `v.float64()`.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `number` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## Jerarquía \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VFloat64`**

## Constructores \{#constructors\}

### constructor \{#constructor\}

• **new VFloat64**&lt;`Type`, `IsOptional`&gt;(`«destructured»`)

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `number` |
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

Solo en TypeScript, el tipo de TS de los valores de JS que valida
este validador.

#### Heredado de \{#inherited-from\}

BaseValidator.type

#### Definido en \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

Solo para TypeScript, si este es un validador de objeto,
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

### kind \{#kind\}

• `Readonly` **kind**: `"float64"`

El tipo de validador, `"float64"`.

#### Definido en \{#defined-in\}

[values/validators.ts:120](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L120)