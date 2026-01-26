---
id: "values.ConvexError"
title: "Clase: ConvexError<TData>"
custom_edit_url: null
---

[values](../modules/values.md).ConvexError

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TData` | extends [`Valor`](../modules/values.md#value) |

## Jerarquía \{#hierarchy\}

* `Error`

  ↳ **`ConvexError`**

## Constructores \{#constructors\}

### constructor \{#constructor\}

• **new ConvexError**&lt;`TData`&gt;(`data`)

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TData` | extends [`Value`](../modules/values.md#value) |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `data` | `TData` |

#### Sobrescrituras \{#overrides\}

Error.constructor

#### Definido en \{#defined-in\}

[values/errors.ts:10](https://github.com/get-convex/convex-js/blob/main/src/values/errors.ts#L10)

## Propiedades \{#properties\}

### stackTraceLimit \{#stacktracelimit\}

▪ `Static` **stackTraceLimit**: `number`

La propiedad `Error.stackTraceLimit` especifica el número de frames de pila
recopilados por un seguimiento de pila (ya sea generado por `new Error().stack` o
`Error.captureStackTrace(obj)`).

El valor predeterminado es `10`, pero se puede establecer en cualquier número válido de JavaScript. Los cambios
afectarán a cualquier seguimiento de pila capturado *después* de que se haya cambiado el valor.

Si se establece en un valor no numérico o en un número negativo, los seguimientos de pila
no capturarán ningún frame.

#### Heredado de \{#inherited-from\}

Error.stackTraceLimit

#### Definido en \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/@types+node@18.19.130/node&#95;modules/@types/node/globals.d.ts:68

***

### cause \{#cause\}

• `Optional` **cause**: `unknown`

#### Heredado de \{#inherited-from\}

Error.cause

#### Definido en \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es2022.error.d.ts:24

***

### message \{#message\}

• **message**: `string`

#### Heredado de \{#inherited-from\}

Error.message

#### Definido en \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es5.d.ts:1055

***

### stack \{#stack\}

• `opcional` **stack**: `string`

#### Heredado de \{#inherited-from\}

Error.stack

#### Definido en \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es5.d.ts:1056

***

### name \{#name\}

• **name**: `string` = `"ConvexError"`

#### Sobrescrituras \{#overrides\}

Error.name

#### Definido en \{#defined-in\}

[values/errors.ts:6](https://github.com/get-convex/convex-js/blob/main/src/values/errors.ts#L6)

***

### data \{#data\}

• **data**: `TData`

#### Definido en \{#defined-in\}

[values/errors.ts:7](https://github.com/get-convex/convex-js/blob/main/src/values/errors.ts#L7)

***

### [IDENTIFYING_FIELD]

• **[IDENTIFYING&#95;FIELD]**: `boolean` = `true`

#### Definido en \{#defined-in\}

[values/errors.ts:8](https://github.com/get-convex/convex-js/blob/main/src/values/errors.ts#L8)

## Métodos \{#methods\}

### captureStackTrace \{#capturestacktrace\}

▸ `Static` **captureStackTrace**(`targetObject`, `constructorOpt?`): `void`

Crea una propiedad `.stack` en `targetObject`, que al acceder a ella devuelve
una cadena que representa la ubicación en el código en la que se llamó a
`Error.captureStackTrace()`.

```js
const myObject = {};
Error.captureStackTrace(myObject);
myObject.stack;  // Similar a `new Error().stack`
```

La primera línea de la traza tendrá el prefijo
`${myObject.name}: ${myObject.message}`.

El argumento opcional `constructorOpt` acepta una función. Si se proporciona, todos los frames
por encima de `constructorOpt`, incluido `constructorOpt`, se omitirán de la
traza de pila generada.

El argumento `constructorOpt` es útil para ocultar al usuario los detalles
de implementación de la generación del error. Por ejemplo:

```js
function a() {
  b();
}

function b() {
  c();
}

function c() {
  // Create an error without stack trace to avoid calculating the stack trace twice.
  const { stackTraceLimit } = Error;
  Error.stackTraceLimit = 0;
  const error = new Error();
  Error.stackTraceLimit = stackTraceLimit;

  // Capture the stack trace above function b
  Error.captureStackTrace(error, b); // Ni la función c ni b están incluidas en el stack trace
  throw error;
}

a();
```

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `targetObject` | `object` |
| `constructorOpt?` | `Function` |

#### Valor de retorno \{#returns\}

`void`

#### Heredado de \{#inherited-from\}

Error.captureStackTrace

#### Definido en \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/@types+node@18.19.130/node&#95;modules/@types/node/globals.d.ts:52

***

### prepareStackTrace \{#preparestacktrace\}

▸ `Static` **prepareStackTrace**(`err`, `stackTraces`): `any`

**`Véase`**

https://v8.dev/docs/stack-trace-api#customizing-stack-traces

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `err` | `Error` |
| `stackTraces` | `CallSite`[] |

#### Devuelve \{#returns\}

`any`

#### Heredado de \{#inherited-from\}

Error.prepareStackTrace

#### Definido en \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/@types+node@18.19.130/node&#95;modules/@types/node/globals.d.ts:56