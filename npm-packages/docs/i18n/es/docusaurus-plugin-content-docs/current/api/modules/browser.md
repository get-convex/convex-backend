---
id: "browser"
title: "Módulo: browser"
custom_edit_url: null
---

Herramientas para acceder a Convex desde el navegador.

**Si estás usando React, utiliza el módulo [react](react.md) en su lugar.**

## Uso \{#usage\}

Crea un [ConvexHttpClient](../classes/browser.ConvexHttpClient.md) para conectarte a Convex Cloud.

```typescript
import { ConvexHttpClient } from "convex/browser";
// normalmente se carga desde una variable de entorno
const address = "https://small-mouse-123.convex.cloud";
const convex = new ConvexHttpClient(address);
```

## Clases \{#classes\}

* [ConvexHttpClient](../classes/browser.ConvexHttpClient.md)
* [ConvexClient](../classes/browser.ConvexClient.md)
* [BaseConvexClient](../classes/browser.BaseConvexClient.md)

## Interfaces \{#interfaces\}

* [BaseConvexClientOptions](../interfaces/browser.BaseConvexClientOptions.md)
* [SubscribeOptions](../interfaces/browser.SubscribeOptions.md)
* [MutationOptions](../interfaces/browser.MutationOptions.md)
* [OptimisticLocalStore](../interfaces/browser.OptimisticLocalStore.md)

## Alias de tipos \{#type-aliases\}

### HttpMutationOptions \{#httpmutationoptions\}

Ƭ **HttpMutationOptions**: `Object`

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `skipQueue` | `boolean` | Omite la cola predeterminada de mutaciones y ejecuta esta mutación de inmediato. Esto permite usar la misma instancia de HttpConvexClient para solicitar múltiples mutaciones en paralelo, algo que no es posible con clientes basados en WebSocket. |

#### Definido en \{#defined-in\}

[browser/http&#95;client.ts:40](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L40)

***

### ConvexClientOptions \{#convexclientoptions\}

Ƭ **ConvexClientOptions**: [`BaseConvexClientOptions`](../interfaces/browser.BaseConvexClientOptions.md) &amp; &#123; `disabled?`: `boolean` ; `unsavedChangesWarning?`: `boolean`  &#125;

#### Definido en \{#defined-in\}

[browser/simple&#95;client.ts:36](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L36)

***

### AuthTokenFetcher \{#authtokenfetcher\}

Ƭ **AuthTokenFetcher**: (`args`: &#123; `forceRefreshToken`: `boolean`  &#125;) =&gt; `Promise`&lt;`string` | `null` | `undefined`&gt;

#### Declaración de tipo \{#type-declaration\}

▸ (`args`): `Promise`&lt;`string` | `null` | `undefined`&gt;

Una función asíncrona que devuelve un JWT. Dependiendo de los proveedores de autenticación
configurados en convex/auth.config.ts, este puede ser un token de identidad OpenID Connect
codificado como JWT o un JWT tradicional.

`forceRefreshToken` es `true` si el servidor rechazó un token devuelto previamente
o si se anticipa que el token expirará pronto
en función de su tiempo `exp`.

Consulta ConvexReactClient.setAuth.

##### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `args` | `Object` |
| `args.forceRefreshToken` | `boolean` |

##### Devuelve \{#returns\}

`Promise`&lt;`string` | `null` | `undefined`&gt;

#### Definido en \{#defined-in\}

[browser/sync/authentication&#95;manager.ts:25](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/authentication_manager.ts#L25)

***

### ConnectionState \{#connectionstate\}

Ƭ **ConnectionState**: `Object`

Estado que describe la conexión del cliente al backend de Convex.

#### Declaración de tipo \{#type-declaration\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `hasInflightRequests` | `boolean` | - |
| `isWebSocketConnected` | `boolean` | - |
| `timeOfOldestInflightRequest` | `Date` | `null` | - |
| `hasEverConnected` | `boolean` | Indica si el cliente ha llegado alguna vez a abrir un WebSocket hasta el estado &quot;ready&quot;. |
| `connectionCount` | `number` | El número de veces que este cliente se ha conectado al backend de Convex. Varias cosas pueden hacer que el cliente se vuelva a conectar: errores del servidor, mala conexión a Internet, expiración de la autenticación. Pero que este número sea alto es una indicación de que el cliente tiene problemas para mantener una conexión estable. |
| `connectionRetries` | `number` | El número de veces que este cliente ha intentado conectarse al backend de Convex sin éxito. |
| `inflightMutations` | `number` | El número de mutaciones que están actualmente en curso. |
| `inflightActions` | `number` | El número de acciones que están actualmente en curso. |

#### Definido en \{#defined-in\}

[browser/sync/client.ts:147](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L147)

***

### FunctionResult \{#functionresult\}

Ƭ **FunctionResult**: `FunctionSuccess` | `FunctionFailure`

El resultado de ejecutar una función en el servidor.

Si la función genera una excepción, tendrá un `errorMessage`. De lo contrario,
producirá un `Value`.

#### Definido en \{#defined-in\}

[browser/sync/function&#95;result.ts:11](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/function_result.ts#L11)

***

### OptimisticUpdate \{#optimisticupdate\}

Ƭ **OptimisticUpdate**&lt;`Args`&gt;: (`localQueryStore`: [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md), `args`: `Args`) =&gt; `void`

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Args` | extends `Record`&lt;`string`, [`Value`](values.md#value)&gt; |

#### Declaración de tipo \{#type-declaration\}

▸ (`localQueryStore`, `args`): `void`

Una actualización temporal y local de los resultados de consultas dentro de este cliente.

Esta actualización siempre se ejecutará cuando una mutación se sincronice con el
servidor de Convex y se revertirá cuando la mutación se complete.

Ten en cuenta que las actualizaciones optimistas se pueden invocar varias veces. Si el cliente
carga nuevos datos mientras la mutación está en curso, la actualización se aplicará
de nuevo.

##### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `localQueryStore` | [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md) | Una interfaz para leer y editar resultados de consultas locales. |
| `args` | `Args` | Los argumentos de la mutación. |

##### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[browser/sync/optimistic&#95;updates.ts:90](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/optimistic_updates.ts#L90)

***

### PaginationStatus \{#paginationstatus\}

Ƭ **PaginationStatus**: `"LoadingFirstPage"` | `"CanLoadMore"` | `"LoadingMore"` | `"Exhausted"`

#### Definido en \{#defined-in\}

[browser/sync/pagination.ts:5](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/pagination.ts#L5)

***

### QueryJournal \{#queryjournal\}

Ƭ **QueryJournal**: `string` | `null`

Una representación serializada de las decisiones tomadas durante la ejecución de una consulta.

Se genera un registro cuando una función de consulta se ejecuta por primera vez y se reutiliza
cuando esa consulta se vuelve a ejecutar.

Actualmente se usa para almacenar cursores finales de paginación para garantizar
que las páginas de consultas paginadas siempre terminen en el mismo cursor. Esto
permite una paginación reactiva sin saltos.

`null` se usa para representar registros vacíos.

#### Definido en \{#defined-in\}

[browser/sync/protocol.ts:113](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/protocol.ts#L113)

***

### QueryToken \{#querytoken\}

Ƭ **QueryToken**: `string` &amp; &#123; `__queryToken`: `true`  &#125;

Una cadena que representa el nombre y los argumentos de una consulta.

Esto se utiliza en [BaseConvexClient](../classes/browser.BaseConvexClient.md).

#### Definido en \{#defined-in\}

[browser/sync/udf&#95;path&#95;utils.ts:31](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/udf_path_utils.ts#L31)

***

### PaginatedQueryToken \{#paginatedquerytoken\}

Ƭ **PaginatedQueryToken**: [`QueryToken`](browser.md#querytoken) &amp; &#123; `__paginatedQueryToken`: `true`  &#125;

Una cadena de texto que representa el nombre y los argumentos de una consulta paginada.

Se trata de una forma especializada de QueryToken que se utiliza para consultas paginadas.

#### Definido en \{#defined-in\}

[browser/sync/udf&#95;path&#95;utils.ts:38](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/udf_path_utils.ts#L38)

***

### UserIdentityAttributes \{#useridentityattributes\}

Ƭ **UserIdentityAttributes**: `Omit`&lt;[`UserIdentity`](../interfaces/server.UserIdentity.md), `"tokenIdentifier"`&gt;

#### Definido en \{#defined-in\}

[server/authentication.ts:215](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L215)