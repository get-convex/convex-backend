---
id: "react.ConvexReactClientOptions"
title: "Interfaz: ConvexReactClientOptions"
custom_edit_url: null
---

[react](../modules/react.md).ConvexReactClientOptions

Opciones para [ConvexReactClient](../classes/react.ConvexReactClient.md).

## Jerarquía \{#hierarchy\}

* [`BaseConvexClientOptions`](browser.BaseConvexClientOptions.md)

  ↳ **`ConvexReactClientOptions`**

## Propiedades \{#properties\}

### unsavedChangesWarning \{#unsavedchangeswarning\}

• `Optional` **unsavedChangesWarning**: `boolean`

Si se debe avisar al usuario de que tiene cambios sin guardar pendientes
al salir de la página o cerrar una página web.

Esto solo es posible cuando el objeto `window` existe, es decir, en un navegador.

El valor predeterminado es `true` en los navegadores.

#### Heredado de \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[unsavedChangesWarning](browser.BaseConvexClientOptions.md#unsavedchangeswarning)

#### Definido en \{#defined-in\}

[browser/sync/client.ts:69](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L69)

***

### webSocketConstructor \{#websocketconstructor\}

• `Opcional` **webSocketConstructor**: `Object`

#### Firma de llamada \{#call-signature\}

• **new webSocketConstructor**(`url`, `protocols?`): `WebSocket`

Especifica un constructor de
[WebSocket](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)
alternativo que se usará para la comunicación del cliente con la nube de Convex.
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

#### Heredado de \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[webSocketConstructor](browser.BaseConvexClientOptions.md#websocketconstructor)

#### Definido en \{#defined-in\}

[browser/sync/client.ts:76](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L76)

***

### verbose \{#verbose\}

• `Optional` **verbose**: `boolean`

Activa registros adicionales para fines de depuración.

El valor predeterminado es `false`.

#### Heredado de \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[verbose](browser.BaseConvexClientOptions.md#verbose)

#### Definido en \{#defined-in\}

[browser/sync/client.ts:82](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L82)

***

### logger \{#logger\}

• `Optional` **logger**: `boolean` | `Logger`

Un `logger`, `true` o `false`. Si no se proporciona o es `true`, se registra en la consola.
Si es `false`, los registros no se muestran en ningún sitio.

Puedes crear tu propio logger para personalizar el registro y enviar los logs a otro lugar.
Un logger es un objeto con 4 métodos: log(), warn(), error() y logVerbose().
Estos métodos pueden recibir múltiples argumentos de cualquier tipo, como console.log().

#### Heredado de \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[logger](browser.BaseConvexClientOptions.md#logger)

#### Definido en \{#defined-in\}

[browser/sync/client.ts:91](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L91)

***

### reportDebugInfoToConvex \{#reportdebuginfotoconvex\}

• `Opcional` **reportDebugInfoToConvex**: `boolean`

Envía métricas adicionales a Convex con fines de depuración.

El valor predeterminado es `false`.

#### Heredado de \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[reportDebugInfoToConvex](browser.BaseConvexClientOptions.md#reportdebuginfotoconvex)

#### Definido en \{#defined-in\}

[browser/sync/client.ts:97](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L97)

***

### onServerDisconnectError \{#onserverdisconnecterror\}

• `Opcional` **onServerDisconnectError**: (`message`: `string`) =&gt; `void`

#### Declaración de tipo \{#type-declaration\}

▸ (`message`): `void`

Esta API es experimental: puede cambiar o desaparecer.

Una función que se llama al recibir mensajes anómalos de cierre de WebSocket
del despliegue de Convex conectado. El contenido de estos mensajes no es estable;
son detalles de implementación que pueden cambiar.

Considera esta API como una solución provisional de observabilidad hasta que haya
códigos de nivel superior con recomendaciones sobre cómo actuar, que podrían ofrecer
una interfaz más estable en lugar de `string`.

Consulta `connectionState` para obtener métricas más cuantitativas sobre el estado de la conexión.

##### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `message` | `string` |

##### Devuelve \{#returns\}

`void`

#### Heredado de \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[onServerDisconnectError](browser.BaseConvexClientOptions.md#onserverdisconnecterror)

#### Definido en \{#defined-in\}

[browser/sync/client.ts:111](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L111)

***

### skipConvexDeploymentUrlCheck \{#skipconvexdeploymenturlcheck\}

• `Optional` **skipConvexDeploymentUrlCheck**: `boolean`

Omite la validación de que la URL de implementación de Convex tenga el formato
`https://happy-animal-123.convex.cloud` o localhost.

Esto puede ser útil si se ejecuta un backend de Convex autoalojado que usa una
URL diferente.

El valor predeterminado es `false`.

#### Heredado de \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[skipConvexDeploymentUrlCheck](browser.BaseConvexClientOptions.md#skipconvexdeploymenturlcheck)

#### Definido en \{#defined-in\}

[browser/sync/client.ts:121](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L121)

***

### authRefreshTokenLeewaySeconds \{#authrefreshtokenleewayseconds\}

• `Optional` **authRefreshTokenLeewaySeconds**: `number`

Si utilizas autenticación, el número de segundos antes de que un token caduque en que debemos renovarlo.

El valor predeterminado es `2`.

#### Heredado de \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[authRefreshTokenLeewaySeconds](browser.BaseConvexClientOptions.md#authrefreshtokenleewayseconds)

#### Definido en \{#defined-in\}

[browser/sync/client.ts:127](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L127)

***

### expectAuth \{#expectauth\}

• `Optional` **expectAuth**: `boolean`

Esta API es experimental: puede cambiar o desaparecer.

Indica si las solicitudes de consulta, mutación y acción deben aplazarse
hasta que se pueda enviar el primer token de autenticación.

Habilitar este comportamiento funciona bien para páginas que solo
deben ser vistas por clientes autenticados.

De forma predeterminada es false; no espera un token de autenticación.

#### Heredado de \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[expectAuth](browser.BaseConvexClientOptions.md#expectauth)

#### Definido en \{#defined-in\}

[browser/sync/client.ts:139](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L139)