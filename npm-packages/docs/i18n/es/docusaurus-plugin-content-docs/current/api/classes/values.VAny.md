---
id: "values.VAny"
title: "Clase: VAny<Type, IsOptional, FieldPaths>"
custom_edit_url: null
---

[values](../modules/values.md).VAny

El tipo de validador `v.any()`.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `any` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `string` |

## Jerarquía \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`, `FieldPaths`&gt;

  ↳ **`VAny`**

## Constructores \{#constructors\}

### constructor \{#constructor\}

• **new VAny**&lt;`Type`, `IsOptional`, `FieldPaths`&gt;(`«destructured»`)

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `any` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |

#### Heredado de \{#inherited-from\}

BaseValidator&lt;Type, IsOptional, FieldPaths&gt;.constructor

#### Definido en \{#defined-in\}

[values/validators.ts:54](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L54)

## Propiedades \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

Solo para TypeScript: el tipo de TS de los valores de JS validados
por este validador.

#### Heredado de \{#inherited-from\}

BaseValidator.type

#### Definido en \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `FieldPaths`

Solo en TypeScript, si se trata de un validador de tipo Object,
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

• `Readonly` **kind**: `"any"`

El tipo de validador, `"any"`.

#### Definido en \{#defined-in\}

[values/validators.ts:261](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L261)