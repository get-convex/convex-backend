---
id: "values.VId"
title: "Clase: VId<Type, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VId

El tipo del validador `v.id(tableName)`.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `Type` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## Jerarquía \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VId`**

## Constructores \{#constructors\}

### constructor \{#constructor\}

• **new VId**&lt;`Type`, `IsOptional`&gt;(`«destructured»`)

Normalmente se usaría `v.id(tableName)` en su lugar.

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
| › `tableName` | `TableNameFromType`&lt;`Type`&gt; |

#### Sobrescribe \{#overrides\}

BaseValidator&lt;Type, IsOptional&gt;.constructor

#### Definido en \{#defined-in\}

[values/validators.ts:84](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L84)

## Propiedades \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

Solo para TypeScript, el tipo de TS correspondiente a los valores de JS validados
por este validador.

#### Heredado de \{#inherited-from\}

BaseValidator.type

#### Definido en \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

Solo para TypeScript, si se trata de un validador Object, entonces
este es el tipo de TypeScript de los nombres de sus propiedades.

#### Heredado de \{#inherited-from\}

BaseValidator.fieldPaths

#### Definido en \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

Indica si se trata de un validador de Valor para una propiedad opcional de un objeto.

#### Heredado de \{#inherited-from\}

BaseValidator.isOptional

#### Definido en \{#defined-in\}

[values/validators.ts:47](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L47)

***

### isConvexValidator \{#isconvexvalidator\}

• `Readonly` **isConvexValidator**: `true`

Siempre es `"true"`.

#### Heredado de \{#inherited-from\}

BaseValidator.isConvexValidator

#### Definido en \{#defined-in\}

[values/validators.ts:52](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L52)

***

### tableName \{#tablename\}

• `Readonly` **tableName**: `TableNameFromType`&lt;`Type`&gt;

El nombre de la tabla a la que deben pertenecer las Id validadas.

#### Definido en \{#defined-in\}

[values/validators.ts:74](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L74)

***

### kind \{#kind\}

• `Readonly` **kind**: `"id"`

El tipo de validador, `"id"`.

#### Definido en \{#defined-in\}

[values/validators.ts:79](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L79)