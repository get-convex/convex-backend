---
id: "browser.BaseConvexClientOptions"
title: "Interfaz: BaseConvexClientOptions"
custom_edit_url: null
---

[browser](../modules/browser.md).BaseConvexClientOptions

Opciones para [BaseConvexClient](../classes/browser.BaseConvexClient.md).

## Jerarquía \{#hierarchy\}

* **`BaseConvexClientOptions`**

  ↳ [`ConvexReactClientOptions`](react.ConvexReactClientOptions.md)

## Propiedades \{#properties\}

### unsavedChangesWarning \{#unsavedchangeswarning\}

• `Optional` **unsavedChangesWarning**: `boolean`

Si se debe preguntar al usuario si tiene cambios sin guardar pendientes
al navegar fuera de una página web o al cerrarla.

Esto solo es posible cuando el objeto `window` existe, es decir, en un navegador.

El valor predeterminado es `true` en navegadores.

#### Definido en \{#defined-in\}

[browser/sync/client.ts:69](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L69)

***

### webSocketConstructor \{#websocketconstructor\}

• `Opcional` **webSocketConstructor**: `Object`

#### Firma de llamada \{#call-signature\}

• **new webSocketConstructor**(`url`, `protocols?`): `WebSocket`

Especifica un constructor de
[WebSocket](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)
alternativo que se utilizará para la comunicación del cliente con la nube de Convex.
El comportamiento predeterminado es usar `WebSocket` del entorno global.

##### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `url` | `string` | `URL` |
| `protocols?` | `string` | `string`[] |

##### Devuelve \{#returns\}

`WebSocket`

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo |
| :------ | :------ |
| `prototype` | `WebSocket` |
| `CONNECTING` | `0` |
| `OPEN` | `1` |
| `CLOSING` | `2` |
| `CLOSED` | `3` |

#### Definido en \{#defined-in\}

[browser/sync/client.ts:76](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L76)

***

### verbose \{#verbose\}

• `Optional` **verbose**: `boolean`

Agrega registros adicionales para fines de depuración.

El valor predeterminado es `false`.

#### Definido en \{#defined-in\}

[browser/sync/client.ts:82](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L82)

***

### logger \{#logger\}

• `Optional` **logger**: `boolean` | `Logger`

Un logger, `true` o `false`. Si no se proporciona o es `true`, se registrará en la consola.
Si es `false`, los registros no se imprimen en ningún lugar.

Puedes construir tu propio logger para personalizar el registro y enviarlo a otro destino.
Un logger es un objeto con 4 métodos: log(), warn(), error() y logVerbose().
Estos métodos pueden recibir múltiples argumentos de cualquier tipo, igual que console.log().

#### Definido en \{#defined-in\}

[browser/sync/client.ts:91](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L91)

***

### reportDebugInfoToConvex \{#reportdebuginfotoconvex\}

• `Optional` **reportDebugInfoToConvex**: `boolean`

Envía métricas adicionales a Convex para depuración.

El valor predeterminado es `false`.

#### Definido en \{#defined-in\}

[browser/sync/client.ts:97](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L97)

***

### onServerDisconnectError \{#onserverdisconnecterror\}

• `Opcional` **onServerDisconnectError**: (`message`: `string`) =&gt; `void`

#### Declaración de tipo \{#type-declaration\}

▸ (`message`): `void`

Esta API es experimental: puede cambiar o desaparecer.

Una función que se llama al recibir mensajes de cierre anormal de WebSocket desde el
despliegue de Convex conectado. El contenido de estos mensajes no es estable,
es un detalle de implementación que puede cambiar.

Considera esta API como una solución provisional para la observabilidad hasta que haya códigos de nivel más alto con
recomendaciones sobre qué hacer, que podrían proporcionar una interfaz más estable
en lugar de `string`.

Consulta `connectionState` para obtener métricas más cuantitativas sobre el estado de la conexión.

##### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `message` | `string` |

##### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[browser/sync/client.ts:111](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L111)

***

### skipConvexDeploymentUrlCheck \{#skipconvexdeploymenturlcheck\}

• `Opcional` **skipConvexDeploymentUrlCheck**: `boolean`

Omitir la validación de que la URL de implementación de Convex tenga el formato
`https://happy-animal-123.convex.cloud` o localhost.

Esto puede ser útil si ejecutas un backend de Convex autohospedado que utiliza una
URL diferente.

El valor predeterminado es `false`.

#### Definido en \{#defined-in\}

[browser/sync/client.ts:121](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L121)

***

### authRefreshTokenLeewaySeconds \{#authrefreshtokenleewayseconds\}

• `Optional` **authRefreshTokenLeewaySeconds**: `number`

Si usas autenticación, el número de segundos antes de que un token caduque en los que deberíamos renovarlo.

El valor predeterminado es `2`.

#### Definido en \{#defined-in\}

[browser/sync/client.ts:127](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L127)

***

### expectAuth \{#expectauth\}

• `Optional` **expectAuth**: `boolean`

Esta API es experimental: puede cambiar o desaparecer.

Indica si las solicitudes de consulta, mutación y acción deben retrasarse
hasta que se pueda enviar el primer token de autenticación.

Activar este comportamiento funciona bien para páginas que
solo deben ser vistas por clientes autenticados.

De forma predeterminada es false, sin esperar un token de autenticación.

#### Definido en \{#defined-in\}

[browser/sync/client.ts:139](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L139)