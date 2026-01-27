---
id: "values.VRecord"
title: "Clase: VRecord<Type, Key, Value, IsOptional, FieldPaths>"
custom_edit_url: null
---

[values](../modules/values.md).VRecord

Tipo del validador `v.record()`.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `Type` |
| `Key` | extends [`Validator`](../modules/values.md#validator)&lt;`string`, `"required"`, `any`&gt; |
| `Value` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `string` |

## Jerarquía \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`, `FieldPaths`&gt;

  ↳ **`VRecord`**

## Constructores \{#constructors\}

### constructor \{#constructor\}

• **new VRecord**&lt;`Type`, `Key`, `Value`, `IsOptional`, `FieldPaths`&gt;(`«destructured»`)

Normalmente usarías `v.record(key, value)` en su lugar.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `Type` |
| `Key` | extends [`Validator`](../modules/values.md#validator)&lt;`string`, `"required"`, `any`&gt; |
| `Value` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |
| › `key` | `Key` |
| › `value` | `Value` |

#### Sobrescribe \{#overrides\}

BaseValidator&lt;Type, IsOptional, FieldPaths&gt;.constructor

#### Definido en \{#defined-in\}

[values/validators.ts:547](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L547)

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

• `Readonly` **fieldPaths**: `FieldPaths`

Solo para TypeScript: si se trata de un validador de tipo `Object`, este es el tipo de TypeScript de los nombres de sus propiedades.

#### Heredado de \{#inherited-from\}

BaseValidator.fieldPaths

#### Definido en \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

Indica si se trata de un validador para el valor de una propiedad de objeto opcional.

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

### key \{#key\}

• `Readonly` **key**: `Key`

Validador de las claves del registro.

#### Definido en \{#defined-in\}

[values/validators.ts:532](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L532)

***

### value \{#value\}

• `Readonly` **value**: `Value`

Validador de los valores del registro.

#### Definido en \{#defined-in\}

[values/validators.ts:537](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L537)

***

### kind \{#kind\}

• `Readonly` **kind**: `"record"`

El tipo de validador, `"record"`.

#### Definido en \{#defined-in\}

[values/validators.ts:542](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L542)