---
id: "server.HttpRouter"
title: "Clase: HttpRouter"
custom_edit_url: null
---

[server](../modules/server.md).HttpRouter

Router HTTP para especificar las rutas y los métodos de [httpActionGeneric](../modules/server.md#httpactiongeneric)

Un archivo de ejemplo `convex/http.js` podría tener este aspecto.

```js
import { httpRouter } from "convex/server";
import { getMessagesByAuthor } from "./getMessagesByAuthor";
import { httpAction } from "./_generated/server";

const http = httpRouter();

// Las acciones HTTP se pueden definir en línea...
http.route({
  path: "/message",
  method: "POST",
  handler: httpAction(async ({ runMutation }, request) => {
    const { author, body } = await request.json();

    await runMutation(api.sendMessage.default, { body, author });
    return new Response(null, {
      status: 200,
    });
  })
});

// ...o se pueden importar desde otros archivos.
http.route({
  path: "/getMessagesByAuthor",
  method: "GET",
  handler: getMessagesByAuthor,
});

// Convex espera que el enrutador sea la exportación por defecto de `convex/http.js`.
export default http;
```

## Constructores \{#constructors\}

### constructor \{#constructor\}

• **new HttpRouter**()

## Propiedades \{#properties\}

### exactRoutes \{#exactroutes\}

• **exactRoutes**: `Map`&lt;`string`, `Map`&lt;`"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, [`PublicHttpAction`](../modules/server.md#publichttpaction)&gt;&gt;

#### Definido en \{#defined-in\}

[server/router.ts:143](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L143)

***

### prefixRoutes \{#prefixroutes\}

• **prefixRoutes**: `Map`&lt;`"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, `Map`&lt;`string`, [`PublicHttpAction`](../modules/server.md#publichttpaction)&gt;&gt;

#### Definido en \{#defined-in\}

[server/router.ts:144](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L144)

***

### isRouter \{#isrouter\}

• **isRouter**: `true`

#### Definido en \{#defined-in\}

[server/router.ts:145](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L145)

## Métodos \{#methods\}

### route \{#route\}

▸ **route**(`spec`): `void`

Especifica una HttpAction que se usará para responder a solicitudes
para un método HTTP (por ejemplo, &quot;GET&quot;) y una ruta o un pathPrefix.

Las rutas deben comenzar con una barra diagonal (/). Los prefijos de ruta también deben terminar en una barra diagonal (/).

```js
// matches `/profile` (but not `/profile/`)
http.route({ path: "/profile", method: "GET", handler: getProfile})

// coincide con `/profiles/`, `/profiles/abc` y `/profiles/a/c/b` (pero no con `/profile`)
http.route({ pathPrefix: "/profile/", method: "GET", handler: getProfile})
```

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `spec` | [`RouteSpec`](../modules/server.md#routespec) |

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[server/router.ts:161](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L161)

***

### getRoutes \{#getroutes\}

▸ **getRoutes**(): readonly [`string`, `"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, [`PublicHttpAction`](../modules/server.md#publichttpaction)][]

Devuelve una lista de acciones HTTP enrutadas.

Se utilizan para completar la lista de rutas que se muestra en la página Functions del panel de control de Convex.

#### Devuelve \{#returns\}

readonly [`string`, `"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, [`PublicHttpAction`](../modules/server.md#publichttpaction)][]

* un array de tuplas [ruta, método, endpoint].

#### Definido en \{#defined-in\}

[server/router.ts:229](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L229)

***

### lookup \{#lookup\}

▸ **lookup**(`path`, `method`): `null` | readonly [[`PublicHttpAction`](../modules/server.md#publichttpaction), `"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, `string`]

Devuelve la acción HTTP apropiada y la ruta y el método de la solicitud asociados.

La ruta y el método devueltos se usan para el registro (logging) y las métricas, y deben
coincidir con una de las rutas devueltas por `getRoutes`.

Por ejemplo,

```js
http.route({ pathPrefix: "/profile/", method: "GET", handler: getProfile});

http.lookup("/profile/abc", "GET") // devuelve [getProfile, "GET", "/profile/*"]
```

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `path` | `string` |
| `method` | `"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"` | `"HEAD"` |

#### Devuelve \{#returns\}

`null` | readonly [[`PublicHttpAction`](../modules/server.md#publichttpaction), `"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, `string`]

* una tupla [[PublicHttpAction](../modules/server.md#publichttpaction), método, ruta] o null.

#### Definido en \{#defined-in\}

[server/router.ts:275](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L275)

***

### runRequest \{#runrequest\}

▸ **runRequest**(`argsStr`, `requestRoute`): `Promise`&lt;`string`&gt;

Dada una representación en forma de cadena JSON de un objeto `Request`, devuelve una `Response`
enrutando la solicitud y ejecutando el endpoint correspondiente o devolviendo
una `Response` 404.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `argsStr` | `string` | Una cadena JSON que representa un objeto `Request`. |
| `requestRoute` | `string` | - |

#### Devuelve \{#returns\}

`Promise`&lt;`string`&gt;

* un objeto Response.

#### Definido en \{#defined-in\}

[server/router.ts:304](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L304)