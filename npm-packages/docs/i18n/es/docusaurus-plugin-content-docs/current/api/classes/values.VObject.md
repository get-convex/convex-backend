---
id: "values.VObject"
title: "Clase: VObject<Type, Fields, IsOptional, FieldPaths>"
custom_edit_url: null
---

[values](../modules/values.md).VObject

Tipo del validador `v.object()`.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `Type` |
| `Fields` | extends `Record`&lt;`string`, [`GenericValidator`](../modules/values.md#genericvalidator)&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = &#123; [Property in keyof Fields]: JoinFieldPaths&lt;Property &amp; string, Fields[Property][&quot;fieldPaths&quot;]&gt; | Property &#125;[keyof `Fields`] &amp; `string` |

## Jerarquía \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`, `FieldPaths`&gt;

  ↳ **`VObject`**

## Constructores \{#constructors\}

### constructor \{#constructor\}

• **new VObject**&lt;`Type`, `Fields`, `IsOptional`, `FieldPaths`&gt;(`«desestructurado»`)

Normalmente usarías `v.object({ ... })` en su lugar.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `Type` |
| `Fields` | extends `Record`&lt;`string`, [`GenericValidator`](../modules/values.md#genericvalidator)&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Fields[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof `Fields`] &amp; `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |
| › `fields` | `Fields` |

#### Sobrescribe \{#overrides\}

BaseValidator&lt;Type, IsOptional, FieldPaths&gt;.constructor

#### Definido en \{#defined-in\}

[values/validators.ts:304](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L304)

## Propiedades \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

Solo para TypeScript, el tipo de TS de los valores de JS que valida
este validador.

#### Heredado de \{#inherited-from\}

BaseValidator.type

#### Definido en \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `FieldPaths`

Solo en TypeScript, si se trata de un validador Object, entonces
este es el tipo de TypeScript de los nombres de sus propiedades.

#### Heredado de \{#inherited-from\}

BaseValidator.fieldPaths

#### Definido en \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

Indica si este es un validador para valores de propiedades de objeto opcionales.

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

### fields \{#fields\}

• `Readonly` **fields**: `Fields`

Un objeto que contiene el validador de cada propiedad.

#### Definido en \{#defined-in\}

[values/validators.ts:294](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L294)

***

### kind \{#kind\}

• `Readonly` **kind**: `"object"`

El tipo de validador, `"object"`.

#### Definido en \{#defined-in\}

[values/validators.ts:299](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L299)

## Métodos \{#methods\}

### omit \{#omit\}

▸ **omit**&lt;`K`&gt;(`...fields`): [`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Type`, `K`&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Fields`, `K`&gt;&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Omit&lt;Fields, K&gt;&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Fields`, `K`&gt;&gt;] &amp; `string`&gt;

Crea un nuevo VObject con los campos especificados omitidos.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `K` | extends `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `...fields` | `K`[] | Los nombres de los campos que se omiten de este VObject. |

#### Devuelve \{#returns\}

[`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Type`, `K`&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Fields`, `K`&gt;&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Omit&lt;Fields, K&gt;&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Fields`, `K`&gt;&gt;] &amp; `string`&gt;

#### Definido en \{#defined-in\}

[values/validators.ts:349](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L349)

***

### pick \{#pick\}

▸ **pick**&lt;`K`&gt;(`...fields`): [`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Type`, `Extract`&lt;keyof `Type`, `K`&gt;&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Fields`, `K`&gt;&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Pick&lt;Fields, K&gt;&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Fields`, `K`&gt;&gt;] &amp; `string`&gt;

Crea un nuevo VObject con únicamente los campos especificados.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `K` | extends `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `...fields` | `K`[] | Los nombres de los campos que se seleccionarán de este VObject. |

#### Devuelve \{#returns\}

[`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Type`, `Extract`&lt;keyof `Type`, `K`&gt;&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Fields`, `K`&gt;&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Pick&lt;Fields, K&gt;&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Fields`, `K`&gt;&gt;] &amp; `string`&gt;

#### Definido en \{#defined-in\}

[values/validators.ts:366](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L366)

***

### partial \{#partial\}

▸ **partial**(): [`VObject`](values.VObject.md)&lt;&#123; [K in string | number | symbol]?: Type[K] &#125;, &#123; [K in string | number | symbol]: VOptional&lt;Fields[K]&gt; &#125;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;&#123; [K in string | number | symbol]: VOptional&lt;Fields[K]&gt; &#125;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof `Fields`] &amp; `string`&gt;

Crea un nuevo VObject en el que todos los campos están marcados como opcionales.

#### Devuelve \{#returns\}

[`VObject`](values.VObject.md)&lt;&#123; [K in string | number | symbol]?: Type[K] &#125;, &#123; [K in string | number | symbol]: VOptional&lt;Fields[K]&gt; &#125;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;&#123; [K in string | number | symbol]: VOptional&lt;Fields[K]&gt; &#125;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof `Fields`] &amp; `string`&gt;

#### Definido en \{#defined-in\}

[values/validators.ts:386](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L386)

***

### extend \{#extend\}

▸ **extend**&lt;`NewFields`&gt;(`fields`): [`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Type` &amp; [`ObjectType`](../modules/values.md#objecttype)&lt;`NewFields`&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Fields` &amp; `NewFields`&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Fields &amp; NewFields&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Fields` &amp; `NewFields`&gt;] &amp; `string`&gt;

Crea un nuevo `VObject` con campos adicionales fusionados.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `NewFields` | extends `Record`&lt;`string`, [`GenericValidator`](../modules/values.md#genericvalidator)&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `fields` | `NewFields` | Un objeto con validadores adicionales para fusionar en este VObject. |

#### Devuelve \{#returns\}

[`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Type` &amp; [`ObjectType`](../modules/values.md#objecttype)&lt;`NewFields`&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Fields` &amp; `NewFields`&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Fields &amp; NewFields&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Fields` &amp; `NewFields`&gt;] &amp; `string`&gt;

#### Definido en \{#defined-in\}

[values/validators.ts:407](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L407)