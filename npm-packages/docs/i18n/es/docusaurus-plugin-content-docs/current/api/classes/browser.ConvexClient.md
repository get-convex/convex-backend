---
id: "browser.ConvexClient"
title: "Clase: ConvexClient"
custom_edit_url: null
---

[browser](../modules/browser.md).ConvexClient

Se suscribe a funciones de consulta de Convex y ejecuta mutaciones y acciones mediante un WebSocket.

Este cliente no proporciona actualizaciones optimistas para las mutaciones.
Los clientes de terceros pueden optar por envolver [BaseConvexClient](browser.BaseConvexClient.md) para tener un mayor control.

```ts
const client = new ConvexClient("https://happy-otter-123.convex.cloud");
const unsubscribe = client.onUpdate(api.messages.list, {}, (messages) => {
  console.log(messages[0].body);
});
```

## Constructores \{#constructors\}

### constructor \{#constructor\}

• **new ConvexClient**(`address`, `options?`)

Crea un cliente e inicia inmediatamente una conexión WebSocket a la dirección proporcionada.

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `address` | `string` |
| `options` | [`ConvexClientOptions`](../modules/browser.md#convexclientoptions) |

#### Definido en \{#defined-in\}

[browser/simple&#95;client.ts:119](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L119)

## Accesores \{#accessors\}

### closed \{#closed\}

• `get` **closed**(): `boolean`

Una vez cerrado, ningún callback registrado volverá a ejecutarse.

#### Devuelve \{#returns\}

`boolean`

#### Definido en \{#defined-in\}

[browser/simple&#95;client.ts:96](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L96)

***

### client \{#client\}

• `get` **client**(): [`BaseConvexClient`](browser.BaseConvexClient.md)

#### Devuelve \{#returns\}

[`BaseConvexClient`](browser.BaseConvexClient.md)

#### Definido en \{#defined-in\}

[browser/simple&#95;client.ts:99](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L99)

***

### disabled \{#disabled\}

• `get` **disabled**(): `boolean`

#### Devuelve \{#returns\}

`boolean`

#### Definido en \{#defined-in\}

[browser/simple&#95;client.ts:110](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L110)

## Métodos \{#methods\}

### onUpdate \{#onupdate\}

▸ **onUpdate**&lt;`Query`&gt;(`query`, `args`, `callback`, `onError?`): `Unsubscribe`&lt;`Query`[`"_returnType"`]&gt;

Invoca un callback cada vez que se recibe un nuevo resultado para una consulta. El callback
se ejecutará poco después de registrarse si ya hay un resultado para la consulta
en memoria.

El valor de retorno es un objeto Unsubscribe que es a la vez una función
y un objeto con propiedades. Ambos de los siguientes patrones funcionan con este objeto:

```ts
// call the return value as a function
const unsubscribe = client.onUpdate(api.messages.list, {}, (messages) => {
  console.log(messages);
});
unsubscribe();

// desempaqueta el valor de retorno en sus propiedades
const {
  getCurrentValue,
  unsubscribe,
} = client.onUpdate(api.messages.list, {}, (messages) => {
  console.log(messages);
});
```

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### Parámetros \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | Un [FunctionReference](../modules/server.md#functionreference) para la consulta pública que se va a ejecutar. |
| `args` | [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Query`&gt; | Los argumentos con los que se ejecutará la consulta. |
| `callback` | (`result`: [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;) =&gt; `unknown` | Función que se llamará cuando se actualice el resultado de la consulta. |
| `onError?` | (`e`: `Error`) =&gt; `unknown` | Función que se llamará cuando el resultado de la consulta se actualice con un error. Si no se proporciona, los errores se lanzarán en lugar de invocar el callback. |

#### Devuelve \{#returns\}

`Unsubscribe`&lt;`Query`[`"_returnType"`]&gt;

una función Unsubscribe para dejar de llamar a la función onUpdate.

#### Definido en \{#defined-in\}

[browser/simple&#95;client.ts:185](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L185)

***

### onPaginatedUpdate_experimental \{#onpaginatedupdate_experimental\}

▸ **onPaginatedUpdate&#95;experimental**&lt;`Query`&gt;(`query`, `args`, `options`, `callback`, `onError?`): `Unsubscribe`&lt;`PaginatedQueryResult`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;[]&gt;&gt;

Invoca un callback cada vez que se recibe un nuevo resultado para una consulta paginada.

Esta es una característica experimental en vista previa: la API final puede cambiar.
En particular, el comportamiento de caché, la división en páginas y las opciones obligatorias de la consulta paginada
pueden cambiar.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `query` | `Query` | Un [FunctionReference](../modules/server.md#functionreference) para la consulta pública que se ejecutará. |
| `args` | [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Query`&gt; | Los argumentos con los que se ejecutará la consulta. |
| `options` | `Object` | Opciones para la consulta paginada, incluidas initialNumItems e id. |
| `options.initialNumItems` | `number` | - |
| `callback` | (`result`: [`PaginationResult`](../interfaces/server.PaginationResult.md)&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;) =&gt; `unknown` | Función que se llama cuando se actualiza el resultado de la consulta. |
| `onError?` | (`e`: `Error`) =&gt; `unknown` | Función que se llama cuando la actualización del resultado de la consulta produce un error. |

#### Returns \{#returns\}

`Unsubscribe`&lt;`PaginatedQueryResult`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;[]&gt;&gt;

una función Unsubscribe para dejar de llamar al callback.

#### Definido en \{#defined-in\}

[browser/simple&#95;client.ts:263](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L263)

***

### close \{#close\}

▸ **close**(): `Promise`&lt;`void`&gt;

#### Devuelve \{#returns\}

`Promise`&lt;`void`&gt;

#### Definido en \{#defined-in\}

[browser/simple&#95;client.ts:366](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L366)

***

### getAuth \{#getauth\}

▸ **getAuth**(): `undefined` | &#123; `token`: `string` ; `decoded`: `Record`&lt;`string`, `any`&gt;  &#125;

Obtiene el token de autenticación JWT actual y las declaraciones de identidad decodificadas.

#### Devuelve \{#returns\}

`undefined` | &#123; `token`: `string` ; `decoded`: `Record`&lt;`string`, `any`&gt;  &#125;

#### Definido en \{#defined-in\}

[browser/simple&#95;client.ts:380](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L380)

***

### setAuth \{#setauth\}

▸ **setAuth**(`fetchToken`, `onChange?`): `void`

Establece el token de autenticación que se utilizará para las consultas y mutaciones posteriores.
Se volverá a llamar automáticamente a `fetchToken` si un token expira.
`fetchToken` debe devolver `null` si no se puede recuperar el token, por ejemplo
cuando se hayan revocado permanentemente los permisos del usuario.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `fetchToken` | [`AuthTokenFetcher`](../modules/browser.md#authtokenfetcher) | una función asíncrona que retorna el JWT (normalmente un OpenID Connect Identity Token) |
| `onChange?` | (`isAuthenticated`: `boolean`) =&gt; `void` | un callback que se llamará cuando cambie el estado de la autenticación |

#### Valor de retorno \{#returns\}

`void`

#### Definido en \{#defined-in\}

[browser/simple&#95;client.ts:393](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L393)

***

### mutation \{#mutation\}

▸ **mutation**&lt;`Mutation`&gt;(`mutation`, `args`, `options?`): `Promise`&lt;`Awaited`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;&gt;

Ejecuta una función de mutación.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | Una [FunctionReference](../modules/server.md#functionreference) para la mutación pública que se va a ejecutar. |
| `args` | [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Mutation`&gt; | Un objeto de argumentos para la mutación. |
| `options?` | [`MutationOptions`](../interfaces/browser.MutationOptions.md) | Un objeto de opciones de tipo [MutationOptions](../interfaces/browser.MutationOptions.md) para la mutación. |

#### Devuelve \{#returns\}

`Promise`&lt;`Awaited`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;&gt;

Una promesa con el resultado de la mutación.

#### Definido en \{#defined-in\}

[browser/simple&#95;client.ts:488](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L488)

***

### acción \{#action\}

▸ **action**&lt;`Action`&gt;(`action`, `args`): `Promise`&lt;`Awaited`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;&gt;

Ejecuta una función de tipo acción.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"action"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `action` | `Action` | Un [FunctionReference](../modules/server.md#functionreference) para ejecutar la acción pública. |
| `args` | [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Action`&gt; | Un objeto de argumentos para la acción. |

#### Devuelve \{#returns\}

`Promise`&lt;`Awaited`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;&gt;

Una promesa del resultado de la acción.

#### Definido en \{#defined-in\}

[browser/simple&#95;client.ts:505](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L505)

***

### query \{#query\}

▸ **query**&lt;`Query`&gt;(`query`, `args`): `Promise`&lt;`Awaited`&lt;`Query`[`"_returnType"`]&gt;&gt;

Obtiene el resultado de una consulta solo una vez.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `query` | `Query` | Un [FunctionReference](../modules/server.md#functionreference) de la consulta pública que se va a ejecutar. |
| `args` | `Query`[`"_args"`] | Un objeto de argumentos para la consulta. |

#### Devuelve \{#returns\}

`Promise`&lt;`Awaited`&lt;`Query`[`"_returnType"`]&gt;&gt;

Una promesa del resultado de la consulta.

#### Definido en \{#defined-in\}

[browser/simple&#95;client.ts:521](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L521)

***

### connectionState \{#connectionstate\}

▸ **connectionState**(): [`ConnectionState`](../modules/browser.md#connectionstate)

Devuelve el [`ConnectionState`](../modules/browser.md#connectionstate) actual entre el cliente y el backend de Convex.

#### Devuelve \{#returns\}

[`ConnectionState`](../modules/browser.md#connectionstate)

El [ConnectionState](../modules/browser.md#connectionstate) del backend de Convex.

#### Definido en \{#defined-in\}

[browser/simple&#95;client.ts:553](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L553)

***

### subscribeToConnectionState \{#subscribetoconnectionstate\}

▸ **subscribeToConnectionState**(`cb`): () =&gt; `void`

Suscríbete al [ConnectionState](../modules/browser.md#connectionstate) entre el cliente y el backend
de Convex, llamando a un callback cada vez que cambie.

Los callbacks suscritos se llamarán cuando cambie cualquier parte de ConnectionState.
ConnectionState puede crecer en versiones futuras (por ejemplo, para proporcionar un array de
solicitudes en curso), en cuyo caso los callbacks se llamarían con mayor frecuencia.

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `cb` | (`connectionState`: [`ConnectionState`](../modules/browser.md#connectionstate)) =&gt; `void` |

#### Devuelve \{#returns\}

`fn`

Una función para cancelar la suscripción y dejar de escuchar.

▸ (): `void`

Se suscribe al [ConnectionState](../modules/browser.md#connectionstate) entre el cliente y el backend de Convex, llamando a un callback cada vez que cambie.

Los callbacks suscritos se llamarán cuando cualquier parte de ConnectionState cambie.
ConnectionState puede crecer en versiones futuras (por ejemplo, para proporcionar un array de
solicitudes en curso), en cuyo caso los callbacks se invocarían con más frecuencia.

##### Devuelve \{#returns\}

`void`

Una función de cancelación de suscripción para dejar de recibir actualizaciones.

#### Definido en \{#defined-in\}

[browser/simple&#95;client.ts:568](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L568)