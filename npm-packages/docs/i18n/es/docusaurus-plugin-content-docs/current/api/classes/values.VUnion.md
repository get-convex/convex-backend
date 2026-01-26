---
id: "values.VUnion"
title: "Clase: VUnion<Type, T, IsOptional, FieldPaths>"
custom_edit_url: null
---

[values](../modules/values.md).VUnion

Tipo del validador `v.union()`.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `Type` |
| `T` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt;[] |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `T`[`number`][`"fieldPaths"`] |

## Jerarquía \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`, `FieldPaths`&gt;

  ↳ **`VUnion`**

## Constructores \{#constructors\}

### constructor \{#constructor\}

• **new VUnion**&lt;`Type`, `T`, `IsOptional`, `FieldPaths`&gt;(`«desestructurado»`)

Normalmente usarías `v.union(...members)` en su lugar.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `Type` |
| `T` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt;[] |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `T`[`number`][`"fieldPaths"`] |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |
| › `members` | `T` |

#### Sobrescribe a \{#overrides\}

BaseValidator&lt;Type, IsOptional, FieldPaths&gt;.constructor

#### Definido en \{#defined-in\}

[values/validators.ts:619](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L619)

## Propiedades \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

Solo en TypeScript, el tipo de TS de los valores de JS validados
por este validador.

#### Heredado de \{#inherited-from\}

BaseValidator.type

#### Definido en \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `FieldPaths`

Solo para TypeScript, si se trata de un validador de tipo objeto, entonces
este es el tipo de TypeScript de los nombres de sus propiedades.

#### Heredado de \{#inherited-from\}

BaseValidator.fieldPaths

#### Definido en \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

Indica si este es un validador para el valor de una propiedad de objeto opcional.

#### Heredado de \{#inherited-from\}

BaseValidator.isOptional

#### Definido en \{#defined-in\}

[values/validators.ts:47](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L47)

***

### isConvexValidator \{#isconvexvalidator\}

• `Readonly` **isConvexValidator**: `true`

Siempre `true`.

#### Heredado de \{#inherited-from\}

BaseValidator.isConvexValidator

#### Definido en \{#defined-in\}

[values/validators.ts:52](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L52)

***

### members \{#members\}

• `Readonly` **members**: `T`

El array de validadores; uno de ellos debe coincidir con el valor.

#### Definido en \{#defined-in\}

[values/validators.ts:609](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L609)

***

### kind \{#kind\}

• `Readonly` **kind**: `"union"`

El tipo de validador, `"union"`.

#### Definido en \{#defined-in\}

[values/validators.ts:614](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L614)