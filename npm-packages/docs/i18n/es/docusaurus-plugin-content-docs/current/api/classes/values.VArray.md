---
id: "values.VArray"
title: "Clase: VArray<Type, Element, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VArray

El tipo de validador `v.array()`.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `Type` |
| `Element` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## Jerarquía \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VArray`**

## Constructores \{#constructors\}

### constructor \{#constructor\}

• **new VArray**&lt;`Type`, `Element`, `IsOptional`&gt;(`«destructured»`)

En general usarás `v.array(element)` en su lugar.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `Type` |
| `Element` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |
| › `element` | `Element` |

#### Sobrescrituras \{#overrides\}

BaseValidator&lt;Type, IsOptional&gt;.constructor

#### Definido en \{#defined-in\}

[values/validators.ts:490](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L490)

## Propiedades \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

Solo en TypeScript, el tipo de TS de los valores de JS que valida este validador.

#### Heredado de \{#inherited-from\}

BaseValidator.type

#### Definido en \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

Solo en TypeScript, si se trata de un validador de tipo Object, entonces
este es el tipo de TS de los nombres de sus propiedades.

#### Heredado de \{#inherited-from\}

BaseValidator.fieldPaths

#### Definido en \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

Indica si se trata de un validador para una propiedad opcional de un objeto.

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

### element \{#element\}

• `Readonly` **element**: `Element`

Validador de los elementos del array.

#### Definido en \{#defined-in\}

[values/validators.ts:480](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L480)

***

### kind \{#kind\}

• `Readonly` **kind**: `"array"`

El tipo de validador, `"array"`.

#### Definido en \{#defined-in\}

[values/validators.ts:485](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L485)