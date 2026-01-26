---
id: "react.ConvexReactClient"
title: "Clase: ConvexReactClient"
custom_edit_url: null
---

[react](../modules/react.md).ConvexReactClient

Un cliente de Convex para usar con React.

Carga consultas reactivas y ejecuta mutaciones a través de un WebSocket.

## Constructores \{#constructors\}

### constructor \{#constructor\}

• **new ConvexReactClient**(`address`, `options?`)

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `address` | `string` | La URL de tu despliegue de Convex, normalmente proporcionada por una variable de entorno. Por ejemplo, `https://small-mouse-123.convex.cloud`. |
| `options?` | [`ConvexReactClientOptions`](../interfaces/react.ConvexReactClientOptions.md) | Consulta [ConvexReactClientOptions](../interfaces/react.ConvexReactClientOptions.md) para una descripción completa. |

#### Definido en \{#defined-in\}

[react/client.ts:317](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L317)

## Accesores \{#accessors\}

### url \{#url\}

• `get` **url**(): `string`

Devuelve la dirección de este cliente, útil para crear un nuevo cliente.

No se garantiza que coincida con la dirección con la que se creó este cliente:
puede estar normalizada.

#### Devuelve \{#returns\}

`string`

#### Definido en \{#defined-in\}

[react/client.ts:352](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L352)

***

### logger \{#logger\}

• `get` **logger**(): `Logger`

Obtiene el logger de este cliente.

#### Devuelve \{#returns\}

`Logger`

El `Logger` de este cliente.

#### Definido en \{#defined-in\}

[react/client.ts:713](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L713)

## Métodos \{#methods\}

### setAuth \{#setauth\}

▸ **setAuth**(`fetchToken`, `onChange?`): `void`

Establece el token de autenticación que se usará para las consultas y mutaciones posteriores.
`fetchToken` se llamará automáticamente de nuevo si un token caduca.
`fetchToken` debe devolver `null` si el token no se puede recuperar, por ejemplo,
cuando se han revocado permanentemente los derechos del usuario.

#### Parámetros \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `fetchToken` | [`AuthTokenFetcher`](../modules/browser.md#authtokenfetcher) | una función asíncrona que devuelve el token de identidad de OpenID Connect codificado como JWT |
| `onChange?` | (`isAuthenticated`: `boolean`) =&gt; `void` | un callback que se ejecutará cuando cambie el estado de autenticación |

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[react/client.ts:408](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L408)

***

### clearAuth \{#clearauth\}

▸ **clearAuth**(): `void`

Borra el token de autenticación actual si está configurado.

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[react/client.ts:430](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L430)

***

### watchQuery \{#watchquery\}

▸ **watchQuery**&lt;`Query`&gt;(`query`, `...argsAndOptions`): [`Watch`](../interfaces/react.Watch.md)&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

Crea un nuevo [Watch](../interfaces/react.Watch.md) sobre una función de consulta de Convex.

**La mayoría del código de la aplicación no debería llamar a este método directamente. En su lugar, utiliza el hook [useQuery](../modules/react.md#usequery).**

Crear un watch no hace nada; un Watch es un objeto sin estado.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `query` | `Query` | Una [FunctionReference](../modules/server.md#functionreference) para la consulta pública que se ejecutará. |
| `...argsAndOptions` | [`ArgsAndOptions`](../modules/server.md#argsandoptions)&lt;`Query`, [`WatchQueryOptions`](../interfaces/react.WatchQueryOptions.md)&gt; | - |

#### Devuelve \{#returns\}

[`Watch`](../interfaces/react.Watch.md)&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

El objeto [Watch](../interfaces/react.Watch.md).

#### Definido en \{#defined-in\}

[react/client.ts:463](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L463)

***

### prewarmQuery \{#prewarmquery\}

▸ **prewarmQuery**&lt;`Query`&gt;(`queryOptions`): `void`

Indica un posible interés futuro en una suscripción a una consulta.

Actualmente, la implementación se suscribe inmediatamente a una consulta. En el futuro, este método
podría priorizar algunas consultas sobre otras, obtener el resultado de la consulta sin suscribirse o
no hacer nada en conexiones de red lentas o en escenarios de alta carga.

Para usar esto en un componente de React, llama a useQuery() e ignora el valor de retorno.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `queryOptions` | `ConvexQueryOptions`&lt;`Query`&gt; &amp; &#123; `extendSubscriptionFor?`: `number`  &#125; | Una consulta (referencia de función de un objeto API) y sus argumentos, además de un `extendSubscriptionFor` opcional que indica durante cuánto tiempo suscribirse a la consulta. |

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[react/client.ts:539](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L539)

***

### mutación \{#mutation\}

▸ **mutation**&lt;`Mutation`&gt;(`mutation`, `...argsAndOptions`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

Ejecuta una función de mutación.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | Un [FunctionReference](../modules/server.md#functionreference) para ejecutar la mutación pública. |
| `...argsAndOptions` | [`ArgsAndOptions`](../modules/server.md#argsandoptions)&lt;`Mutation`, [`MutationOptions`](../interfaces/react.MutationOptions.md)&lt;[`FunctionArgs`](../modules/server.md#functionargs)&lt;`Mutation`&gt;&gt;&gt; | - |

#### Returns \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

Una promesa que se resuelve con el resultado de la mutación.

#### Definido en \{#defined-in\}

[react/client.ts:618](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L618)

***

### action \{#action\}

▸ **action**&lt;`Action`&gt;(`action`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

Ejecuta una acción.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"action"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `action` | `Action` | Una [FunctionReference](../modules/server.md#functionreference) para la acción pública que se va a ejecutar. |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Action`&gt; | Un objeto de argumentos para la acción. Si se omite, los argumentos serán `{}`. |

#### Returns \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

Una promesa con el resultado de la acción.

#### Definido en \{#defined-in\}

[react/client.ts:639](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L639)

***

### query \{#query\}

▸ **query**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

Obtiene el resultado de una consulta una única vez.

**La mayoría del código de la aplicación debería, en cambio, suscribirse a consultas usando el hook [useQuery](../modules/react.md#usequery).**

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `query` | `Query` | Una [FunctionReference](../modules/server.md#functionreference) para la consulta pública que se va a ejecutar. |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; | Un objeto de argumentos para la consulta. Si se omite, los argumentos serán `{}`. |

#### Devuelve \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

Una promesa que se resuelve con el resultado de la consulta.

#### Definido en \{#defined-in\}

[react/client.ts:659](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L659)

***

### connectionState \{#connectionstate\}

▸ **connectionState**(): [`ConnectionState`](../modules/browser.md#connectionstate)

Obtiene el [`ConnectionState`](../modules/browser.md#connectionstate) actual entre el cliente y el backend de Convex.

#### Devuelve \{#returns\}

[`ConnectionState`](../modules/browser.md#connectionstate)

El [ConnectionState](../modules/browser.md#connectionstate) del backend de Convex.

#### Definido en \{#defined-in\}

[react/client.ts:686](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L686)

***

### subscribeToConnectionState \{#subscribetoconnectionstate\}

▸ **subscribeToConnectionState**(`cb`): () =&gt; `void`

Suscríbete al [ConnectionState](../modules/browser.md#connectionstate) entre el cliente y el backend de
Convex, llamando a un callback cada vez que cambie.

Los callbacks suscritos se invocarán cuando cualquier parte de ConnectionState cambie.
ConnectionState puede ampliarse en futuras versiones (por ejemplo, para proporcionar un array de
solicitudes en curso), en cuyo caso los callbacks se invocarían con más frecuencia.
ConnectionState también puede *perder* propiedades en futuras versiones a medida que determinemos
qué información es más útil. Por lo tanto, esta API se considera inestable.

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `cb` | (`connectionState`: [`ConnectionState`](../modules/browser.md#connectionstate)) =&gt; `void` |

#### Returns \{#returns\}

`fn`

Una función de cancelación de suscripción para dejar de escuchar.

▸ (): `void`

Se suscribe al [ConnectionState](../modules/browser.md#connectionstate) entre el cliente y el backend de Convex, llamando a un callback cada vez que cambie.

Los callbacks suscritos se invocarán cuando cualquier parte de ConnectionState cambie.
ConnectionState puede crecer en versiones futuras (por ejemplo, para proporcionar un array de
peticiones en curso), en cuyo caso los callbacks se invocarían con más frecuencia.
ConnectionState también puede *perder* propiedades en versiones futuras a medida que determinemos
qué información es más útil. Por lo tanto, esta API se considera inestable.

##### Devuelve \{#returns\}

`void`

Una función para anular la suscripción y dejar de escuchar.

#### Definido en \{#defined-in\}

[react/client.ts:702](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L702)

***

### close \{#close\}

▸ **close**(): `Promise`&lt;`void`&gt;

Cierra cualquier conexión de red asociada con este cliente y detiene todas las suscripciones.

Llama a este método cuando termines de usar un [ConvexReactClient](react.ConvexReactClient.md) para
liberar sus sockets y recursos.

#### Returns \{#returns\}

`Promise`&lt;`void`&gt;

Una `Promise` que se resuelve cuando la conexión se ha cerrado por completo.

#### Definido en \{#defined-in\}

[react/client.ts:725](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L725)