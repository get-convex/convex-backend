---
id: "browser.BaseConvexClient"
title: "Clase: BaseConvexClient"
custom_edit_url: null
---

[browser](../modules/browser.md).BaseConvexClient

Cliente de bajo nivel para integrar directamente bibliotecas de gestión de estado
con Convex.

La mayoría de los desarrolladores deberían usar clientes de más alto nivel, como
[ConvexHttpClient](browser.ConvexHttpClient.md) o el [ConvexReactClient](react.ConvexReactClient.md) basado en hooks de React.

## Constructores \{#constructors\}

### constructor \{#constructor\}

• **new BaseConvexClient**(`address`, `onTransition`, `options?`)

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `address` | `string` | La URL de tu despliegue de Convex, a menudo proporcionada por una variable de entorno. Por ejemplo, `https://small-mouse-123.convex.cloud`. |
| `onTransition` | (`updatedQueries`: [`QueryToken`](../modules/browser.md#querytoken)[]) =&gt; `void` | Un callback que recibe un array de tokens de consulta correspondientes a resultados de consultas que han cambiado; se pueden añadir controladores adicionales mediante `addOnTransitionHandler`. |
| `options?` | [`BaseConvexClientOptions`](../interfaces/browser.BaseConvexClientOptions.md) | Consulta [BaseConvexClientOptions](../interfaces/browser.BaseConvexClientOptions.md) para una descripción completa. |

#### Definido en \{#defined-in\}

[browser/sync/client.ts:277](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L277)

## Accesores \{#accessors\}

### url \{#url\}

• `get` **url**(): `string`

Devuelve la dirección de este cliente, útil para crear un nuevo cliente.

No se garantiza que coincida con la dirección con la que se creó este cliente:
puede estar normalizada.

#### Devuelve \{#returns\}

`string`

#### Definido en \{#defined-in\}

[browser/sync/client.ts:1037](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L1037)

## Métodos \{#methods\}

### getMaxObservedTimestamp \{#getmaxobservedtimestamp\}

▸ **getMaxObservedTimestamp**(): `undefined` | `Long`

#### Devuelve \{#returns\}

`undefined` | `Long`

#### Definido en \{#defined-in\}

[browser/sync/client.ts:542](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L542)

***

### addOnTransitionHandler \{#addontransitionhandler\}

▸ **addOnTransitionHandler**(`fn`): () =&gt; `boolean`

Agrega un controlador que se llamará cuando ocurra una transición.

Cualquier efecto secundario externo (por ejemplo, establecer el estado de React) debe manejarse aquí.

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `fn` | (`transition`: `Transition`) =&gt; `void` |

#### Devuelve \{#returns\}

`fn`

▸ (): `boolean`

##### Devuelve \{#returns\}

`boolean`

#### Definido en \{#defined-in\}

[browser/sync/client.ts:621](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L621)

***

### getCurrentAuthClaims \{#getcurrentauthclaims\}

▸ **getCurrentAuthClaims**(): `undefined` | &#123; `token`: `string` ; `decoded`: `Record`&lt;`string`, `any`&gt;  &#125;

Obtiene el JWT de autenticación actual y las declaraciones (claims) decodificadas.

#### Devuelve \{#returns\}

`undefined` | &#123; `token`: `string` ; `decoded`: `Record`&lt;`string`, `any`&gt;  &#125;

#### Definido en \{#defined-in\}

[browser/sync/client.ts:630](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L630)

***

### setAuth \{#setauth\}

▸ **setAuth**(`fetchToken`, `onChange`): `void`

Establece el token de autenticación que se utilizará para las consultas y mutaciones posteriores.
`fetchToken` se llamará de nuevo automáticamente si un token expira.
`fetchToken` debe devolver `null` si no se puede recuperar el token, por ejemplo
cuando los permisos del usuario se hayan revocado permanentemente.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `fetchToken` | [`AuthTokenFetcher`](../modules/browser.md#authtokenfetcher) | una función asíncrona que devuelve el token de identidad de OpenID Connect codificado como JWT |
| `onChange` | (`isAuthenticated`: `boolean`) =&gt; `void` | una función de retorno (callback) que se invocará cuando cambie el estado de autenticación |

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[browser/sync/client.ts:655](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L655)

***

### hasAuth \{#hasauth\}

▸ **hasAuth**(): `boolean`

#### Devuelve \{#returns\}

`boolean`

#### Definido en \{#defined-in\}

[browser/sync/client.ts:662](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L662)

***

### clearAuth \{#clearauth\}

▸ **clearAuth**(): `void`

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[browser/sync/client.ts:672](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L672)

***

### subscribe \{#subscribe\}

▸ **subscribe**(`name`, `args?`, `options?`): `Object`

Se suscribe a una función de consulta.

Cada vez que cambie el resultado de esta consulta, se llamará a la función de callback `onTransition`
pasada al constructor.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `name` | `string` | El nombre de la consulta. |
| `args?` | `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; | Un objeto de argumentos para la consulta. Si se omite, los argumentos serán `{}`. |
| `options?` | [`SubscribeOptions`](../interfaces/browser.SubscribeOptions.md) | Un objeto de opciones [`SubscribeOptions`](../interfaces/browser.SubscribeOptions.md) para esta consulta. |

#### Devuelve \{#returns\}

`Object`

Un objeto que contiene un [QueryToken](../modules/browser.md#querytoken) correspondiente a esta
consulta y una función de devolución de llamada `unsubscribe`.

| Nombre | Tipo |
| :------ | :------ |
| `queryToken` | [`QueryToken`](../modules/browser.md#querytoken) |
| `unsubscribe` | () =&gt; `void` |

#### Definido en \{#defined-in\}

[browser/sync/client.ts:691](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L691)

***

### localQueryResult \{#localqueryresult\}

▸ **localQueryResult**(`udfPath`, `args?`): `undefined` | [`Value`](../modules/values.md#value)

Un resultado de una consulta basado solo en el estado local actual.

La única manera de que esto devuelva un valor es que ya estemos suscritos a la
consulta o que su valor se haya establecido de forma optimista.

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `udfPath` | `string` |
| `args?` | `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; |

#### Devuelve \{#returns\}

`undefined` | [`Valor`](../modules/values.md#value)

#### Definido en \{#defined-in\}

[browser/sync/client.ts:724](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L724)

***

### queryJournal \{#queryjournal\}

▸ **queryJournal**(`name`, `args?`): `undefined` | [`QueryJournal`](../modules/browser.md#queryjournal)

Obtiene el [QueryJournal](../modules/browser.md#queryjournal) actual para esta función de consulta.

Si aún no hemos recibido un resultado para esta consulta, esto será `undefined`.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `name` | `string` | El nombre de la consulta. |
| `args?` | `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; | El objeto de argumentos de esta consulta. |

#### Devuelve \{#returns\}

`undefined` | [`QueryJournal`](../modules/browser.md#queryjournal)

El [QueryJournal](../modules/browser.md#queryjournal) de la consulta o `undefined`.

#### Definido en \{#defined-in\}

[browser/sync/client.ts:777](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L777)

***

### connectionState \{#connectionstate\}

▸ **connectionState**(): [`ConnectionState`](../modules/browser.md#connectionstate)

Devuelve el [ConnectionState](../modules/browser.md#connectionstate) actual entre el cliente y el backend de Convex.

#### Devuelve \{#returns\}

[`ConnectionState`](../modules/browser.md#connectionstate)

El [`ConnectionState`](../modules/browser.md#connectionstate) de la conexión con el backend de Convex.

#### Definido en \{#defined-in\}

[browser/sync/client.ts:792](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L792)

***

### subscribeToConnectionState \{#subscribetoconnectionstate\}

▸ **subscribeToConnectionState**(`cb`): () =&gt; `void`

Se suscribe al [ConnectionState](../modules/browser.md#connectionstate) entre el cliente y el backend de Convex, llamando a una función de *callback* cada vez que cambie.

Las funciones de *callback* suscritas se llamarán cuando cambie cualquier parte de ConnectionState.
ConnectionState puede crecer en versiones futuras (por ejemplo, para proporcionar un array de solicitudes en curso), en cuyo caso se llamaría a los *callbacks* con mayor frecuencia.

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `cb` | (`connectionState`: [`ConnectionState`](../modules/browser.md#connectionstate)) =&gt; `void` |

#### Devuelve \{#returns\}

`fn`

Una función de desuscripción para dejar de escuchar.

▸ (): `void`

Suscríbete al [ConnectionState](../modules/browser.md#connectionstate) entre el cliente y el backend de Convex,
llamando a un callback cada vez que cambie.

Los callbacks suscritos se llamarán cuando cualquier parte de ConnectionState cambie.
ConnectionState puede crecer en versiones futuras (por ejemplo, para proporcionar un array de
solicitudes en curso), en cuyo caso los callbacks se llamarían con más frecuencia.

##### Devuelve \{#returns\}

`void`

Una función para cancelar la suscripción y dejar de escuchar.

#### Definido en \{#defined-in\}

[browser/sync/client.ts:838](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L838)

***

### mutación \{#mutation\}

▸ **mutation**(`name`, `args?`, `options?`): `Promise`&lt;`any`&gt;

Ejecuta una función de mutación.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `name` | `string` | El nombre de la mutación. |
| `args?` | `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; | Un objeto de argumentos para la mutación. Si se omite, los argumentos serán `{}`. |
| `options?` | [`MutationOptions`](../interfaces/browser.MutationOptions.md) | Un objeto de opciones de tipo [MutationOptions](../interfaces/browser.MutationOptions.md) para esta mutación. |

#### Devuelve \{#returns\}

`Promise`&lt;`any`&gt;

* Una promesa con el resultado de la mutación.

#### Definido en \{#defined-in\}

[browser/sync/client.ts:858](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L858)

***

### acción \{#action\}

▸ **action**(`name`, `args?`): `Promise`&lt;`any`&gt;

Ejecuta una acción.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `name` | `string` | El nombre de la acción. |
| `args?` | `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; | Un objeto que contiene los argumentos de la acción. Si se omite, los argumentos serán `{}`. |

#### Devuelve \{#returns\}

`Promise`&lt;`any`&gt;

Una promesa del resultado de la acción.

#### Definido en \{#defined-in\}

[browser/sync/client.ts:979](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L979)

***

### close \{#close\}

▸ **close**(): `Promise`&lt;`void`&gt;

Cierra cualquier conexión de red asociada con este cliente y detiene todas las suscripciones.

Llama a este método cuando termines de usar un [BaseConvexClient](browser.BaseConvexClient.md) para
liberar sus sockets y recursos.

#### Devuelve \{#returns\}

`Promise`&lt;`void`&gt;

Una `Promise` que se cumple cuando la conexión se ha cerrado por completo.

#### Definido en \{#defined-in\}

[browser/sync/client.ts:1026](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L1026)