---
id: "browser.ConvexHttpClient"
title: "Clase: ConvexHttpClient"
custom_edit_url: null
---

[browser](../modules/browser.md).ConvexHttpClient

Un cliente de Convex que ejecuta consultas y mutaciones mediante HTTP.

Este cliente es con estado (posee credenciales de usuario y pone mutaciones en cola),
así que ten cuidado de no compartirlo entre peticiones en un servidor.

Es apropiado para código del lado del servidor (como Lambdas de Netlify) o
aplicaciones web no reactivas.

## Constructores \{#constructors\}

### constructor \{#constructor\}

• **new ConvexHttpClient**(`address`, `options?`)

Crea una nueva instancia de [ConvexHttpClient](browser.ConvexHttpClient.md).

#### Parámetros \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `address` | `string` | La URL de tu despliegue de Convex, a menudo proporcionada por una variable de entorno. Por ejemplo, `https://small-mouse-123.convex.cloud`. |
| `options?` | `Object` | Un objeto de opciones. - `skipConvexDeploymentUrlCheck` - Omite validar que la URL de implementación de Convex tenga el formato `https://happy-animal-123.convex.cloud` o localhost. Esto puede ser útil si ejecutas un backend de Convex autoalojado que usa una URL diferente. - `logger` - Un logger o un booleano. Si no se proporciona, escribe los logs en la consola. Puedes construir tu propio logger para personalizar el registro y enviar los logs a otro lugar o no registrar nada, o usar `false` como abreviatura de un logger que no hace nada (no-op). Un logger es un objeto con 4 métodos: log(), warn(), error() y logVerbose(). Estos métodos pueden recibir múltiples argumentos de cualquier tipo, igual que console.log(). - `auth` - Un JWT que contiene declaraciones (claims) de identidad accesibles en las funciones de Convex. Esta identidad puede caducar, por lo que puede ser necesario llamar a `setAuth()` más adelante, pero para clientes de corta duración es conveniente especificar este valor aquí. - `fetch` - Una implementación personalizada de fetch que se usará para todas las solicitudes HTTP realizadas por este cliente. |
| `options.skipConvexDeploymentUrlCheck?` | `boolean` | - |
| `options.logger?` | `boolean` | `Logger` | - |
| `options.auth?` | `string` | - |
| `options.fetch?` | (`input`: `URL` | `RequestInfo`, `init?`: `RequestInit`) =&gt; `Promise`&lt;`Response`&gt;(`input`: `string` | `URL` | `Request`, `init?`: `RequestInit`) =&gt; `Promise`&lt;`Response`&gt; | - |

#### Definido en \{#defined-in\}

[browser/http&#95;client.ts:97](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L97)

## Accesores \{#accessors\}

### url \{#url\}

• `get` **url**(): `string`

Devuelve la dirección (URL) de este cliente, útil para crear un nuevo cliente.

No se garantiza que coincida con la dirección con la que se creó este cliente:
puede haberse convertido a su forma canónica.

#### Devuelve \{#returns\}

`string`

#### Definido en \{#defined-in\}

[browser/http&#95;client.ts:147](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L147)

## Métodos \{#methods\}

### backendUrl \{#backendurl\}

▸ **backendUrl**(): `string`

Obtiene la URL del backend del [ConvexHttpClient](browser.ConvexHttpClient.md).

**`Obsoleto`**

Usa url, que devuelve la URL sin /api al final.

#### Returns \{#returns\}

`string`

La URL del backend de Convex, que incluye la versión de la API del cliente.

#### Definido en \{#defined-in\}

[browser/http&#95;client.ts:137](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L137)

***

### setAuth \{#setauth\}

▸ **setAuth**(`value`): `void`

Establece el token de autenticación que se usará para las siguientes consultas y mutaciones.

Debe llamarse cada vez que el token cambie (es decir, debido a vencimiento o renovación).

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `value` | `string` | Token de identidad de OpenID Connect codificado en JWT. |

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[browser/http&#95;client.ts:158](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L158)

***

### clearAuth \{#clearauth\}

▸ **clearAuth**(): `void`

Elimina el token de autenticación actual, si está definido.

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[browser/http&#95;client.ts:184](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L184)

***

### consistentQuery \{#consistentquery\}

▸ **consistentQuery**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

Esta API es experimental: puede cambiar o desaparecer.

Ejecuta una función de consulta de Convex en la misma marca de tiempo que
todas las demás ejecuciones de consultas consistentes realizadas por este cliente HTTP.

Esto no es adecuado para ConvexHttpClients de larga duración, ya que los
backends de Convex solo pueden leer hasta un límite hacia el pasado: más
de 30 segundos en el pasado puede que no estén disponibles.

Crea un cliente nuevo para usar un tiempo consistente.

**`Deprecated`**

Esta API es experimental: puede cambiar o desaparecer.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `query` | `Query` | - |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; | El objeto de argumentos de la consulta. Si se omite, se usarán `{}` como argumentos. |

#### Returns \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

Una promesa con el resultado de la consulta.

#### Definido en \{#defined-in\}

[browser/http&#95;client.ts:226](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L226)

***

### query \{#query\}

▸ **query**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

Ejecuta una función de consulta de Convex.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extiende [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `query` | `Query` | - |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; | El objeto de argumentos de la consulta. Si se omite, los argumentos serán `{}`. |

#### Devuelve \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

Una promesa que se resuelve con el resultado de la consulta.

#### Definido en \{#defined-in\}

[browser/http&#95;client.ts:270](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L270)

***

### mutación \{#mutation\}

▸ **mutation**&lt;`Mutation`&gt;(`mutation`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

Ejecuta una función de mutación de Convex. Las mutaciones se encolan de forma predeterminada.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | - |
| `...args` | [`ArgsAndOptions`](../modules/server.md#argsandoptions)&lt;`Mutation`, [`HttpMutationOptions`](../modules/browser.md#httpmutationoptions)&gt; | El objeto de argumentos de la mutación. Si se omite, los argumentos serán `{}`. |

#### Returns \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

Promesa del resultado de la mutación.

#### Definido en \{#defined-in\}

[browser/http&#95;client.ts:430](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L430)

***

### action \{#action\}

▸ **action**&lt;`Action`&gt;(`action`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

Ejecuta una función de acción de Convex. Las acciones no se ponen en cola.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"action"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `action` | `Action` | - |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Action`&gt; | El objeto de argumentos para la acción. Si se omite, los argumentos serán `{}`. |

#### Devuelve \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

Una promesa con el resultado de la acción.

#### Definido en \{#defined-in\}

[browser/http&#95;client.ts:453](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L453)