---
id: "nextjs"
title: "Módulo: nextjs"
custom_edit_url: null
---

Funciones auxiliares para integrar Convex en aplicaciones de Next.js usando renderizado del lado del servidor.

Este módulo contiene:

1. [preloadQuery](nextjs.md#preloadquery), para precargar datos para componentes de cliente reactivos.
2. [fetchQuery](nextjs.md#fetchquery), [fetchMutation](nextjs.md#fetchmutation) y [fetchAction](nextjs.md#fetchaction) para cargar y mutar datos de Convex
   desde Next.js Server Components, Server Actions y Route Handlers.

## Uso \{#usage\}

Todas las funciones exportadas suponen que se ha configurado una URL de implementación de Convex en la variable de entorno `NEXT_PUBLIC_CONVEX_URL`. `npx convex dev` la establecerá automáticamente durante el desarrollo local.

### Precarga de datos \{#preloading-data\}

Precarga los datos dentro de un Server Component:

```typescript
import { preloadQuery } from "convex/nextjs";
import { api } from "@/convex/_generated/api";
import ClientComponent from "./ClientComponent";

export async function ServerComponent() {
  const preloaded = await preloadQuery(api.foo.baz);
  return <ClientComponent preloaded={preloaded} />;
}
```

Y pásalo a un componente de cliente:

```typescript
import { Preloaded, usePreloadedQuery } from "convex/react";
import { api } from "@/convex/_generated/api";

export function ClientComponent(props: {
  preloaded: Preloaded<typeof api.foo.baz>;
}) {
  const data = usePreloadedQuery(props.preloaded);
  // renderiza `data`...
}
```

## Alias de tipo \{#type-aliases\}

### NextjsOptions \{#nextjsoptions\}

Ƭ **NextjsOptions**: `Object`

Opciones para [preloadQuery](nextjs.md#preloadquery), [fetchQuery](nextjs.md#fetchquery), [fetchMutation](nextjs.md#fetchmutation) y [fetchAction](nextjs.md#fetchaction).

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `token?` | `string` | El token de autenticación OpenID Connect codificado como JWT que se usará para la llamada a la función. |
| `url?` | `string` | La URL del despliegue de Convex que se usará para la llamada a la función. De forma predeterminada es `process.env.NEXT_PUBLIC_CONVEX_URL` si no se proporciona. Pasar explícitamente `undefined` aquí (por ejemplo, debido a variables de entorno faltantes) generará un error en el futuro. |
| `skipConvexDeploymentUrlCheck?` | `boolean` | Omitir la validación de que la URL de implementación de Convex tenga el formato `https://happy-animal-123.convex.cloud` o localhost. Esto puede ser útil si se ejecuta un backend de Convex autohospedado que usa una URL diferente. El valor predeterminado es `false`. |

#### Definido en \{#defined-in\}

[nextjs/index.ts:60](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L60)

## Funciones \{#functions\}

### preloadQuery \{#preloadquery\}

▸ **preloadQuery**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`Preloaded`](react.md#preloaded)&lt;`Query`&gt;&gt;

Ejecuta una función de consulta de Convex y devuelve un payload `Preloaded`
que se puede pasar a [usePreloadedQuery](react.md#usepreloadedquery) en un Client Component.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `query` | `Query` | Un [FunctionReference](server.md#functionreference) para la consulta pública que se va a ejecutar, como `api.dir1.dir2.filename.func`. |
| `...args` | [`ArgsAndOptions`](server.md#argsandoptions)&lt;`Query`, [`NextjsOptions`](nextjs.md#nextjsoptions)&gt; | El objeto de argumentos para la consulta. Si se omite, los argumentos serán `{}`. |

#### Devuelve \{#returns\}

`Promise`&lt;[`Preloaded`](react.md#preloaded)&lt;`Query`&gt;&gt;

Una promesa que se resuelve en la carga útil `Preloaded`.

#### Definido en \{#defined-in\}

[nextjs/index.ts:101](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L101)

***

### preloadedQueryResult \{#preloadedqueryresult\}

▸ **preloadedQueryResult**&lt;`Query`&gt;(`preloaded`): [`FunctionReturnType`](server.md#functionreturntype)&lt;`Query`&gt;

Devuelve el resultado de una consulta ejecutada mediante [preloadQuery](nextjs.md#preloadquery).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `preloaded` | [`Preloaded`](react.md#preloaded)&lt;`Query`&gt; | El payload `Preloaded` devuelto por [preloadQuery](nextjs.md#preloadquery). |

#### Devuelve \{#returns\}

[`FunctionReturnType`](server.md#functionreturntype)&lt;`Query`&gt;

El resultado de la consulta.

#### Definido en \{#defined-in\}

[nextjs/index.ts:120](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L120)

***

### fetchQuery \{#fetchquery\}

▸ **fetchQuery**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Query`&gt;&gt;

Ejecuta una función de consulta en Convex.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### Parámetros \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | Una [FunctionReference](server.md#functionreference) para la consulta pública que se va a ejecutar, como `api.dir1.dir2.filename.func`. |
| `...args` | [`ArgsAndOptions`](server.md#argsandoptions)&lt;`Query`, [`NextjsOptions`](nextjs.md#nextjsoptions)&gt; | El objeto de argumentos para la consulta. Si se omite, los argumentos serán `{}`. |

#### Devuelve \{#returns\}

`Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Query`&gt;&gt;

Una promesa que se resuelve con el resultado de la consulta.

#### Definido en \{#defined-in\}

[nextjs/index.ts:136](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L136)

***

### fetchMutation \{#fetchmutation\}

▸ **fetchMutation**&lt;`Mutation`&gt;(`mutation`, `...args`): `Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

Ejecuta una función de mutación de Convex.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](server.md#functionreference)&lt;`"mutation"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | Una [FunctionReference](server.md#functionreference) para la mutación pública que se va a ejecutar, como `api.dir1.dir2.filename.func`. |
| `...args` | [`ArgsAndOptions`](server.md#argsandoptions)&lt;`Mutation`, [`NextjsOptions`](nextjs.md#nextjsoptions)&gt; | El objeto de argumentos para la mutación. Si se omite, los argumentos serán `{}`. |

#### Devuelve \{#returns\}

`Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

Una promesa del resultado de la mutación.

#### Definido en \{#defined-in\}

[nextjs/index.ts:155](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L155)

***

### fetchAction \{#fetchaction\}

▸ **fetchAction**&lt;`Action`&gt;(`action`, `...args`): `Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Action`&gt;&gt;

Ejecuta una función de acción de Convex.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Action` | extiende [`FunctionReference`](server.md#functionreference)&lt;`"action"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `action` | `Action` | Un [FunctionReference](server.md#functionreference) para la acción pública que se ejecutará, como `api.dir1.dir2.filename.func`. |
| `...args` | [`ArgsAndOptions`](server.md#argsandoptions)&lt;`Action`, [`NextjsOptions`](nextjs.md#nextjsoptions)&gt; | El objeto de argumentos para la acción. Si se omite, los argumentos serán `{}`. |

#### Devuelve \{#returns\}

`Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Action`&gt;&gt;

Una promesa que se resuelve con el resultado de la acción.

#### Definido en \{#defined-in\}

[nextjs/index.ts:176](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L176)