---
id: "react"
title: "Módulo: react"
custom_edit_url: null
---

Herramientas para integrar Convex en aplicaciones de React.

Este módulo contiene:

1. [ConvexReactClient](../classes/react.ConvexReactClient.md), un cliente para usar Convex en React.
2. [ConvexProvider](react.md#convexprovider), un componente que almacena este cliente en el contexto de React.
3. Componentes auxiliares de autenticación [Authenticated](react.md#authenticated), [Unauthenticated](react.md#unauthenticated) y [AuthLoading](react.md#authloading).
4. hooks [useQuery](react.md#usequery), [useMutation](react.md#usemutation), [useAction](react.md#useaction) y más para acceder a este
   cliente desde tus componentes de React.

## Uso \{#usage\}

### Creación del cliente \{#creating-the-client\}

```typescript
import { ConvexReactClient } from "convex/react";

// normalmente se carga desde una variable de entorno
const address = "https://small-mouse-123.convex.cloud"
const convex = new ConvexReactClient(address);
```

### Almacenar el cliente en el contexto de React \{#storing-the-client-in-react-context\}

```typescript
import { ConvexProvider } from "convex/react";

<ConvexProvider client={convex}>
  <App />
</ConvexProvider>
```

### Uso de los helpers de autenticación \{#using-the-auth-helpers\}

```typescript
import { Authenticated, Unauthenticated, AuthLoading } from "convex/react";

<Authenticated>
  Logged in
</Authenticated>
<Unauthenticated>
  Logged out
</Unauthenticated>
<AuthLoading>
  Still loading
</AuthLoading>
```

### Uso de los hooks de React \{#using-react-hooks\}

```typescript
import { useQuery, useMutation } from "convex/react";
import { api } from "../convex/_generated/api";

function App() {
  const counter = useQuery(api.getCounter.default);
  const increment = useMutation(api.incrementCounter.default);
  // ¡Tu componente va aquí!
}
```

## Clases \{#classes\}

* [ConvexReactClient](../classes/react.ConvexReactClient.md)

## Interfaces \{#interfaces\}

* [ReactMutation](../interfaces/react.ReactMutation.md)
* [ReactAction](../interfaces/react.ReactAction.md)
* [Watch](../interfaces/react.Watch.md)
* [WatchQueryOptions](../interfaces/react.WatchQueryOptions.md)
* [MutationOptions](../interfaces/react.MutationOptions.md)
* [ConvexReactClientOptions](../interfaces/react.ConvexReactClientOptions.md)

## Referencias \{#references\}

### AuthTokenFetcher \{#authtokenfetcher\}

Reexporta [AuthTokenFetcher](browser.md#authtokenfetcher)

## Alias de tipos \{#type-aliases\}

### ConvexAuthState \{#convexauthstate\}

Ƭ **ConvexAuthState**: `Object`

Tipo que representa el estado de una integración de autenticación con Convex.

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo |
| :------ | :------ |
| `isLoading` | `boolean` |
| `isAuthenticated` | `boolean` |

#### Definido en \{#defined-in\}

[react/ConvexAuthState.tsx:26](https://github.com/get-convex/convex-js/blob/main/src/react/ConvexAuthState.tsx#L26)

***

### OptionalRestArgsOrSkip \{#optionalrestargsorskip\}

Ƭ **OptionalRestArgsOrSkip**&lt;`FuncRef`&gt;: `FuncRef`[`"_args"`] extends `EmptyObject` ? [args?: EmptyObject | &quot;skip&quot;] : [args: FuncRef[&quot;&#95;args&quot;] | &quot;skip&quot;]

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `FuncRef` | extiende [`FunctionReference`](server.md#functionreference)&lt;`any`&gt; |

#### Definido en \{#defined-in\}

[react/client.ts:799](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L799)

***

### Preloaded \{#preloaded\}

Ƭ **Preloaded**&lt;`Query`&gt;: `Object`

La carga de la consulta precargada, que se debe pasar a un componente de cliente
y luego a [usePreloadedQuery](react.md#usepreloadedquery).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo |
| :------ | :------ |
| `__type` | `Query` |
| `_name` | `string` |
| `_argsJSON` | `string` |
| `_valueJSON` | `string` |

#### Definido en \{#defined-in\}

[react/hydration.tsx:12](https://github.com/get-convex/convex-js/blob/main/src/react/hydration.tsx#L12)

***

### PaginatedQueryReference \{#paginatedqueryreference\}

Ƭ **PaginatedQueryReference**: [`FunctionReference`](server.md#functionreference)&lt;`"query"`, `"public"`, &#123; `paginationOpts`: [`PaginationOptions`](../interfaces/server.PaginationOptions.md)  &#125;, [`PaginationResult`](../interfaces/server.PaginationResult.md)&lt;`any`&gt;&gt;

Un [FunctionReference](server.md#functionreference) que puede usarse con [usePaginatedQuery](react.md#usepaginatedquery).

Esta referencia de función debe:

* Referirse a una consulta pública
* Tener un argumento llamado &quot;paginationOpts&quot; de tipo [PaginationOptions](../interfaces/server.PaginationOptions.md)
* Tener un tipo devuelto de [PaginationResult](../interfaces/server.PaginationResult.md).

#### Definido en \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:31](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L31)

***

### UsePaginatedQueryResult \{#usepaginatedqueryresult\}

Ƭ **UsePaginatedQueryResult**&lt;`Item`&gt;: &#123; `results`: `Item`[] ; `loadMore`: (`numItems`: `number`) =&gt; `void`  &#125; &amp; &#123; `status`: `"LoadingFirstPage"` ; `isLoading`: `true`  &#125; | &#123; `status`: `"CanLoadMore"` ; `isLoading`: `false`  &#125; | &#123; `status`: `"LoadingMore"` ; `isLoading`: `true`  &#125; | &#123; `status`: `"Exhausted"` ; `isLoading`: `false`  &#125;

El resultado de llamar al hook [usePaginatedQuery](react.md#usepaginatedquery).

Esto incluye:

* `results` - Un array con los resultados cargados actualmente.
* `isLoading` - Indica si el hook está cargando resultados en este momento.
* `status` - El status de la paginación. Los posibles valores de status son:
  * &quot;LoadingFirstPage&quot;: El hook está cargando la primera página de resultados.
  * &quot;CanLoadMore&quot;: Esta consulta puede tener más elementos por obtener. Llama a `loadMore` para
    obtener otra página.
  * &quot;LoadingMore&quot;: Actualmente se está cargando otra página de resultados.
  * &quot;Exhausted&quot;: Se ha paginado hasta el final de la lista.
* `loadMore(n)` Un callback para obtener más resultados. Solo obtendrá más
  resultados si el status es &quot;CanLoadMore&quot;.

#### Parámetros de tipo \{#type-parameters\}

| Nombre |
| :------ |
| `Item` |

#### Definido en \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:479](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L479)

***

### PaginationStatus \{#paginationstatus\}

Ƭ **PaginationStatus**: [`UsePaginatedQueryResult`](react.md#usepaginatedqueryresult)&lt;`any`&gt;[`"status"`]

Los posibles valores del estado de paginación en [UsePaginatedQueryResult](react.md#usepaginatedqueryresult).

Se trata de una unión de tipos literales de cadena.

#### Definido en \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:507](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L507)

***

### PaginatedQueryArgs \{#paginatedqueryargs\}

Ƭ **PaginatedQueryArgs**&lt;`Query`&gt;: [`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt;

Dado un [PaginatedQueryReference](react.md#paginatedqueryreference), obtiene el tipo del objeto de argumentos de la consulta, excluyendo el argumento `paginationOpts`.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### Definido en \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:515](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L515)

***

### PaginatedQueryItem \{#paginatedqueryitem\}

Ƭ **PaginatedQueryItem**&lt;`Query`&gt;: [`FunctionReturnType`](server.md#functionreturntype)&lt;`Query`&gt;[`"page"`][`number`]

Dada una [PaginatedQueryReference](react.md#paginatedqueryreference), obtiene el tipo del elemento paginado.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extiende de [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### Definido en \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:524](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L524)

***

### UsePaginatedQueryReturnType \{#usepaginatedqueryreturntype\}

Ƭ **UsePaginatedQueryReturnType**&lt;`Query`&gt;: [`UsePaginatedQueryResult`](react.md#usepaginatedqueryresult)&lt;[`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt;&gt;

El tipo de retorno de [usePaginatedQuery](react.md#usepaginatedquery).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extiende [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### Definido en \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:532](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L532)

***

### RequestForQueries \{#requestforqueries\}

Ƭ **RequestForQueries**: `Record`&lt;`string`, &#123; `query`: [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; ; `args`: `Record`&lt;`string`, [`Value`](values.md#value)&gt;  &#125;&gt;

Un objeto que representa una solicitud para cargar múltiples consultas.

Las claves de este objeto son identificadores y los valores son objetos que contienen
la función de consulta y los argumentos que se le pasan.

Se usa como argumento de [useQueries](react.md#usequeries).

#### Definido en \{#defined-in\}

[react/use&#95;queries.ts:137](https://github.com/get-convex/convex-js/blob/main/src/react/use_queries.ts#L137)

## Funciones \{#functions\}

### useConvexAuth \{#useconvexauth\}

▸ **useConvexAuth**(): `Object`

Obtiene el [ConvexAuthState](react.md#convexauthstate) dentro de un componente de React.

Esto requiere que exista un proveedor de integración de autenticación de Convex por encima en el árbol de componentes de React.

#### Devuelve \{#returns\}

`Object`

El [ConvexAuthState](react.md#convexauthstate) actual.

| Nombre | Tipo |
| :------ | :------ |
| `isLoading` | `boolean` |
| `isAuthenticated` | `boolean` |

#### Definido en \{#defined-in\}

[react/ConvexAuthState.tsx:43](https://github.com/get-convex/convex-js/blob/main/src/react/ConvexAuthState.tsx#L43)

***

### ConvexProviderWithAuth \{#convexproviderwithauth\}

▸ **ConvexProviderWithAuth**(`«destructured»`): `Element`

Un reemplazo de [ConvexProvider](react.md#convexprovider) que además proporciona
[ConvexAuthState](react.md#convexauthstate) a los descendientes de este componente.

Úsalo para integrar cualquier proveedor de autenticación con Convex. La prop `useAuth`
debe ser un hook de React que devuelva el estado de autenticación del proveedor
y una función para obtener un token de acceso JWT.

Si la función de la prop `useAuth` se actualiza provocando un nuevo renderizado, entonces el estado de autenticación
pasará a estado de carga y la función `fetchAccessToken()` se llamará de nuevo.

Consulta [Integración de autenticación personalizada](https://docs.convex.dev/auth/advanced/custom-auth) para obtener más información.

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children?` | `ReactNode` |
| › `client` | `IConvexReactClient` |
| › `useAuth` | () =&gt; &#123; `isLoading`: `boolean` ; `isAuthenticated`: `boolean` ; `fetchAccessToken`: (`args`: &#123; `forceRefreshToken`: `boolean`  &#125;) =&gt; `Promise`&lt;`null` | `string`&gt;  &#125; |

#### Devuelve \{#returns\}

`Element`

#### Definido en \{#defined-in\}

[react/ConvexAuthState.tsx:75](https://github.com/get-convex/convex-js/blob/main/src/react/ConvexAuthState.tsx#L75)

***

### Authenticated \{#authenticated\}

▸ **Authenticated**(`«destructured»`): `null` | `Element`

Renderiza sus elementos hijos si el cliente está autenticado.

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children` | `ReactNode` |

#### Devuelve \{#returns\}

`null` | `Element`

#### Definido en \{#defined-in\}

[react/auth&#95;helpers.tsx:10](https://github.com/get-convex/convex-js/blob/main/src/react/auth_helpers.tsx#L10)

***

### Unauthenticated \{#unauthenticated\}

▸ **Unauthenticated**(`«destructured»`): `null` | `Element`

Renderiza los elementos hijos si el cliente usa autenticación pero no está autenticado.

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children` | `ReactNode` |

#### Devuelve \{#returns\}

`null` | `Element`

#### Definido en \{#defined-in\}

[react/auth&#95;helpers.tsx:23](https://github.com/get-convex/convex-js/blob/main/src/react/auth_helpers.tsx#L23)

***

### AuthLoading \{#authloading\}

▸ **AuthLoading**(`«destructured»`): `null` | `Element`

Renderiza los hijos si el cliente no está usando autenticación o está en proceso
de autenticación.

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children` | `ReactNode` |

#### Devuelve \{#returns\}

`null` | `Element`

#### Definido en \{#defined-in\}

[react/auth&#95;helpers.tsx:37](https://github.com/get-convex/convex-js/blob/main/src/react/auth_helpers.tsx#L37)

***

### useConvex \{#useconvex\}

▸ **useConvex**(): [`ConvexReactClient`](../classes/react.ConvexReactClient.md)

Obtén el [ConvexReactClient](../classes/react.ConvexReactClient.md) dentro de un componente de React.

Esto depende de que el [ConvexProvider](react.md#convexprovider) se encuentre por encima en el árbol de componentes de React.

#### Devuelve \{#returns\}

[`ConvexReactClient`](../classes/react.ConvexReactClient.md)

El objeto [ConvexReactClient](../classes/react.ConvexReactClient.md) activo, o `undefined`.

#### Definido en \{#defined-in\}

[react/client.ts:774](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L774)

***

### ConvexProvider \{#convexprovider\}

▸ **ConvexProvider**(`props`, `deprecatedLegacyContext?`): `null` | `ReactElement`&lt;`any`, `any`&gt;

Proporciona a los descendientes de este componente un [ConvexReactClient](../classes/react.ConvexReactClient.md) activo.

Envuelve tu aplicación con este componente para usar los hooks de Convex `useQuery`,
`useMutation` y `useConvex`.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `props` | `Object` | Un objeto con una propiedad `client` que se refiere a un [ConvexReactClient](../classes/react.ConvexReactClient.md). |
| `props.client` | [`ConvexReactClient`](../classes/react.ConvexReactClient.md) | - |
| `props.children?` | `ReactNode` | - |
| `deprecatedLegacyContext?` | `any` | **`Obsoleto`** **`Consulte`** la [documentación de React](https://legacy.reactjs.org/docs/legacy-context.html#referencing-context-in-lifecycle-methods) |

#### Devuelve \{#returns\}

`null` | `ReactElement`&lt;`any`, `any`&gt;

#### Definido en \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/@types+react@18.3.26/node&#95;modules/@types/react/ts5.0/index.d.ts:1129

***

### useQuery \{#usequery\}

▸ **useQuery**&lt;`Query`&gt;(`query`, `...args`): `Query`[`"_returnType"`] | `undefined`

Carga una consulta reactiva dentro de un componente de React.

Este hook de React contiene estado interno que hará que el componente se vuelva a renderizar
cada vez que cambie el resultado de la consulta.

Lanza un error si no se usa dentro de [ConvexProvider](react.md#convexprovider).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### Parámetros \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | Una [FunctionReference](server.md#functionreference) para la consulta pública que se va a ejecutar, como `api.dir1.dir2.filename.func`. |
| `...args` | [`OptionalRestArgsOrSkip`](react.md#optionalrestargsorskip)&lt;`Query`&gt; | Los argumentos de la función de consulta o la cadena `"skip"` si no se debe cargar la consulta. |

#### Devuelve \{#returns\}

`Query`[`"_returnType"`] | `undefined`

el resultado de la consulta. Si la consulta todavía se está cargando, devuelve `undefined`.

#### Definido en \{#defined-in\}

[react/client.ts:820](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L820)

***

### useMutation \{#usemutation\}

▸ **useMutation**&lt;`Mutation`&gt;(`mutation`): [`ReactMutation`](../interfaces/react.ReactMutation.md)&lt;`Mutation`&gt;

Crea un nuevo [`ReactMutation`](../interfaces/react.ReactMutation.md).

Los objetos de mutación se pueden invocar como funciones para solicitar la ejecución de la
función de Convex correspondiente, o configurarse con mayor detalle mediante
[actualizaciones optimistas](https://docs.convex.dev/using/optimistic-updates).

El valor devuelto por este hook es estable entre renderizados, por lo que se puede usar
en arrays de dependencias de React y en lógica de memoización que dependa de la identidad
del objeto sin provocar re-renderizados adicionales.

Lanza un error si se usa fuera de [ConvexProvider](react.md#convexprovider).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Mutation` | se extiende de [`FunctionReference`](server.md#functionreference)&lt;`"mutation"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | Una [FunctionReference](server.md#functionreference) para ejecutar una mutación pública, como `api.dir1.dir2.filename.func`. |

#### Devuelve \{#returns\}

[`ReactMutation`](../interfaces/react.ReactMutation.md)&lt;`Mutation`&gt;

El objeto [`ReactMutation`](../interfaces/react.ReactMutation.md) con ese nombre.

#### Definido en \{#defined-in\}

[react/client.ts:872](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L872)

***

### useAction \{#useaction\}

▸ **useAction**&lt;`Action`&gt;(`action`): [`ReactAction`](../interfaces/react.ReactAction.md)&lt;`Action`&gt;

Crea un nuevo [`ReactAction`](../interfaces/react.ReactAction.md).

Los objetos de acción se pueden llamar como funciones para solicitar la ejecución
de la función de Convex correspondiente.

El valor devuelto por este hook es estable entre renderizados, por lo que puede usarse
en arrays de dependencias de React y en lógica de memoización que depende de la identidad
del objeto sin provocar nuevos renderizados.

Lanza un error si no se usa dentro de [ConvexProvider](react.md#convexprovider).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](server.md#functionreference)&lt;`"action"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `action` | `Action` | Un [FunctionReference](server.md#functionreference) para ejecutar la acción pública, por ejemplo `api.dir1.dir2.filename.func`. |

#### Devuelve \{#returns\}

[`ReactAction`](../interfaces/react.ReactAction.md)&lt;`Action`&gt;

El objeto [ReactAction](../interfaces/react.ReactAction.md) correspondiente a ese nombre.

#### Definido en \{#defined-in\}

[react/client.ts:913](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L913)

***

### useConvexConnectionState \{#useconvexconnectionstate\}

▸ **useConvexConnectionState**(): [`ConnectionState`](browser.md#connectionstate)

Hook de React para obtener el [ConnectionState](browser.md#connectionstate) actual y suscribirse a sus cambios.

Este hook devuelve el estado de la conexión actual y se vuelve a renderizar automáticamente
cuando cualquier parte del estado de la conexión cambia (por ejemplo, al pasar a estar en línea/fuera de línea,
cuando las solicitudes comienzan/terminan, etc.).

La estructura de ConnectionState puede cambiar en el futuro, lo que podría hacer que este
hook se vuelva a renderizar con más frecuencia.

Lanza un error si no se usa dentro de [ConvexProvider](react.md#convexprovider).

#### Devuelve \{#returns\}

[`ConnectionState`](browser.md#connectionstate)

El [ConnectionState](browser.md#connectionstate) actual del backend de Convex.

#### Definido en \{#defined-in\}

[react/client.ts:952](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L952)

***

### usePreloadedQuery \{#usepreloadedquery\}

▸ **usePreloadedQuery**&lt;`Query`&gt;(`preloadedQuery`): `Query`[`"_returnType"`]

Carga una consulta reactiva dentro de un componente de React usando un payload `Preloaded`
proveniente de un Server Component devuelto por [preloadQuery](nextjs.md#preloadquery).

Este hook de React contiene estado interno que provocará un nuevo renderizado
cada vez que cambie el resultado de la consulta.

Lanza un error si no se usa dentro de [ConvexProvider](react.md#convexprovider).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `preloadedQuery` | [`Preloaded`](react.md#preloaded)&lt;`Query`&gt; | El payload de la consulta `Preloaded` de un Server Component. |

#### Devuelve \{#returns\}

`Query`[`"_returnType"`]

el resultado de la consulta. Inicialmente devuelve el resultado obtenido
por el Server Component. Luego devuelve el resultado obtenido por el cliente.

#### Definido en \{#defined-in\}

[react/hydration.tsx:34](https://github.com/get-convex/convex-js/blob/main/src/react/hydration.tsx#L34)

***

### usePaginatedQuery \{#usepaginatedquery\}

▸ **usePaginatedQuery**&lt;`Query`&gt;(`query`, `args`, `options`): [`UsePaginatedQueryReturnType`](react.md#usepaginatedqueryreturntype)&lt;`Query`&gt;

Carga datos de forma reactiva desde una consulta paginada para crear una lista cada vez mayor.

Esto se puede usar para implementar interfaces de usuario de &quot;scroll infinito&quot;.

Este hook debe usarse con referencias de consultas públicas que coincidan con
[PaginatedQueryReference](react.md#paginatedqueryreference).

`usePaginatedQuery` concatena todas las páginas de resultados en una sola lista
y administra los cursores de continuación al solicitar más elementos.

Ejemplo de uso:

```typescript
const { results, status, isLoading, loadMore } = usePaginatedQuery(
  api.messages.list,
  { channel: "#general" },
  { initialNumItems: 5 }
);
```

Si la referencia de la consulta o los argumentos cambian, el estado de paginación se restablecerá
a la primera página. De manera similar, si alguna de las páginas genera un error
InvalidCursor o un error asociado con demasiados datos, el estado de paginación también
se restablecerá a la primera página.

Para obtener más información sobre la paginación, consulta [Paginated Queries](https://docs.convex.dev/database/pagination).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `query` | `Query` | Una `FunctionReference` a la función de consulta pública que se va a ejecutar. |
| `args` | `"skip"` | [`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt; | El objeto de argumentos para la función de consulta, excluyendo la propiedad `paginationOpts`. Esa propiedad la inyecta este hook. |
| `options` | `Object` | Un objeto que especifica `initialNumItems`, el número de elementos que se cargarán en la primera página. |
| `options.initialNumItems` | `number` | - |

#### Devuelve \{#returns\}

[`UsePaginatedQueryReturnType`](react.md#usepaginatedqueryreturntype)&lt;`Query`&gt;

Un [UsePaginatedQueryResult](react.md#usepaginatedqueryresult) que incluye los elementos actualmente cargados, el estado de la paginación y una función `loadMore`.

#### Definido en \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:162](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L162)

***

### resetPaginationId \{#resetpaginationid\}

▸ **resetPaginationId**(): `void`

Restablece el identificador de paginación solo para pruebas, para que estas sepan cuál es.

#### Valor de retorno \{#returns\}

`void`

#### Definido en \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:458](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L458)

***

### optimisticallyUpdateValueInPaginatedQuery \{#optimisticallyupdatevalueinpaginatedquery\}

▸ **optimisticallyUpdateValueInPaginatedQuery**&lt;`Query`&gt;(`localStore`, `query`, `args`, `updateValue`): `void`

Actualiza de forma optimista los valores en una lista paginada.

Esta actualización optimista está diseñada para actualizar datos cargados con
[usePaginatedQuery](react.md#usepaginatedquery). Actualiza la lista aplicando
`updateValue` a cada elemento de la lista en todas las páginas cargadas.

Esto solo se aplicará a consultas con el mismo nombre y los mismos argumentos.

Ejemplo de uso:

```ts
const myMutation = useMutation(api.myModule.myMutation)
.withOptimisticUpdate((localStore, mutationArg) => {

  // Actualiza optimistamente el documento con ID `mutationArg`
  // para que tenga una propiedad adicional.

  optimisticallyUpdateValueInPaginatedQuery(
    localStore,
    api.myModule.paginatedQuery
    {},
    currentValue => {
      if (mutationArg === currentValue._id) {
        return {
          ...currentValue,
          "newProperty": "newValue",
        };
      }
      return currentValue;
    }
  );

});
```

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extiende [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `localStore` | [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md) | Un [OptimisticLocalStore](../interfaces/browser.OptimisticLocalStore.md) que se actualizará. |
| `query` | `Query` | Un [FunctionReference](server.md#functionreference) para la consulta paginada que se actualizará. |
| `args` | [`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt; | El objeto de argumentos de la función de consulta, excluyendo la propiedad `paginationOpts`. |
| `updateValue` | (`currentValue`: [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt;) =&gt; [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt; | Una función que genera los nuevos valores. |

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:578](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L578)

***

### insertAtTop \{#insertattop\}

▸ **insertAtTop**&lt;`Query`&gt;(`options`): `void`

Actualiza una consulta paginada para insertar un elemento al principio de la lista.

Esto es independiente del orden de ordenación, así que si la lista está en orden descendente,
el elemento insertado se tratará como el elemento &quot;más grande&quot;, pero si está en orden
ascendente, se tratará como el &quot;más pequeño&quot;.

Ejemplo:

```ts
const createTask = useMutation(api.tasks.create)
  .withOptimisticUpdate((localStore, mutationArgs) => {
  insertAtTop({
    paginatedQuery: api.tasks.list,
    argsToMatch: { listId: mutationArgs.listId },
    localQueryStore: localStore,
    item: { _id: crypto.randomUUID() as Id<"tasks">, title: mutationArgs.title, completed: false },
  });
});
```

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extiende de [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `options` | `Object` | - |
| `options.paginatedQuery` | `Query` | Una referencia a la función de consulta paginada. |
| `options.argsToMatch?` | `Partial`&lt;[`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt;&gt; | Argumentos opcionales que deben estar presentes en cada consulta paginada relevante. Esto es útil si usas la misma función de consulta con diferentes argumentos para cargar listas distintas. |
| `options.localQueryStore` | [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md) |  |
| `options.item` | [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt; | El elemento que se va a insertar. |

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:640](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L640)

***

### insertAtBottomIfLoaded \{#insertatbottomifloaded\}

▸ **insertAtBottomIfLoaded**&lt;`Query`&gt;(`options`): `void`

Actualiza una consulta paginada para insertar un elemento al final de la lista.

Esto sucede independientemente del orden de ordenación, de modo que si la lista está en orden descendente,
el elemento insertado se tratará como el elemento &quot;más pequeño&quot;, pero si está
en orden ascendente, se tratará como el &quot;más grande&quot;.

Esto solo tiene efecto si la última página está cargada, ya que de otro modo resultaría
en que el elemento se inserte al final de lo que esté cargado (que está en medio de la lista)
y luego desaparezca una vez que termine la actualización optimista.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extiende [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `options` | `Object` | - |
| `options.paginatedQuery` | `Query` | Una referencia a la función de consulta paginada. |
| `options.argsToMatch?` | `Partial`&lt;[`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt;&gt; | Argumentos opcionales que deben estar presentes en cada consulta paginada relevante. Esto es útil si usas la misma función de consulta con diferentes argumentos para cargar distintas listas. |
| `options.localQueryStore` | [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md) |  |
| `options.item` | [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt; | - |

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:689](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L689)

***

### insertAtPosition \{#insertatposition\}

▸ **insertAtPosition**&lt;`Query`&gt;(`options`): `void`

Esta es una función auxiliar para insertar un elemento en una posición específica en una consulta paginada.

Debes proporcionar el sortOrder y una función para obtener la clave de ordenación (un array de valores) a partir de un elemento de la lista.

Esto solo funcionará si la consulta del servidor usa el mismo orden y la misma clave de ordenación que la actualización optimista.

Ejemplo:

```ts
const createTask = useMutation(api.tasks.create)
  .withOptimisticUpdate((localStore, mutationArgs) => {
  insertAtPosition({
    paginatedQuery: api.tasks.listByPriority,
    argsToMatch: { listId: mutationArgs.listId },
    sortOrder: "asc",
    sortKeyFromItem: (item) => [item.priority, item._creationTime],
    localQueryStore: localStore,
    item: {
      _id: crypto.randomUUID() as Id<"tasks">,
      _creationTime: Date.now(),
      title: mutationArgs.title,
      completed: false,
      priority: mutationArgs.priority,
    },
  });
});
```

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `options` | `Object` | - |
| `options.paginatedQuery` | `Query` | Una referencia a la función de consulta paginada. |
| `options.argsToMatch?` | `Partial`&lt;[`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt;&gt; | Argumentos opcionales que deben incluirse en cada consulta paginada relevante. Esto es útil si usas la misma función de consulta con diferentes argumentos para cargar distintas listas. |
| `options.sortOrder` | `"asc"` | `"desc"` | El orden de la consulta paginada (&quot;asc&quot; o &quot;desc&quot;). |
| `options.sortKeyFromItem` | (`element`: [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt;) =&gt; [`Value`](values.md#value) | [`Value`](values.md#value)[] | Una función para derivar la clave de ordenación (un array de valores) a partir de un elemento de la lista. Se recomienda incluir un campo de desempate como `_creationTime`. |
| `options.localQueryStore` | [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md) |  |
| `options.item` | [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt; | El elemento que se va a insertar. |

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:770](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L770)

***

### usePaginatedQuery_experimental \{#usepaginatedquery_experimental\}

▸ **usePaginatedQuery&#95;experimental**&lt;`Query`&gt;(`query`, `args`, `options`): [`UsePaginatedQueryReturnType`](react.md#usepaginatedqueryreturntype)&lt;`Query`&gt;

Nueva implementación experimental de usePaginatedQuery que reemplazará a la actual
en el futuro.

Cargar datos de forma reactiva desde una consulta paginada para crear una lista creciente.

Esta es una implementación alternativa que se basa en una nueva lógica de paginación en el cliente.

Se puede usar para implementar interfaces de usuario con &quot;scroll infinito&quot;.

Este hook debe usarse con referencias de consulta públicas que coincidan con
[PaginatedQueryReference](react.md#paginatedqueryreference).

`usePaginatedQuery` concatena todas las páginas de resultados en una única lista
y gestiona los cursores de continuación al solicitar más elementos.

Ejemplo de uso:

```typescript
const { results, status, isLoading, loadMore } = usePaginatedQuery(
  api.messages.list,
  { channel: "#general" }, // canal de ejemplo
  { initialNumItems: 5 }
);
```

Si cambian la referencia de la consulta o los argumentos, el estado de paginación se restablecerá
a la primera página. De manera similar, si alguna de las páginas produce un error `InvalidCursor`
o un error debido a demasiados datos, el estado de paginación también se
restablecerá a la primera página.

Para obtener más información sobre la paginación, consulta [Consultas paginadas](https://docs.convex.dev/database/pagination).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extiende [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### Parámetros \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | Una FunctionReference a la función de consulta pública que se debe ejecutar. |
| `args` | `"skip"` | [`PaginatedQueryArgs`](react.md#paginatedqueryargs)&lt;`Query`&gt; | El objeto de argumentos de la función de consulta, excluyendo la propiedad `paginationOpts`. Esta propiedad la inyecta este hook. |
| `options` | `Object` | Un objeto que especifica el `initialNumItems` que se cargará en la primera página. |
| `options.initialNumItems` | `number` | - |

#### Devuelve \{#returns\}

[`UsePaginatedQueryReturnType`](react.md#usepaginatedqueryreturntype)&lt;`Query`&gt;

Un [UsePaginatedQueryResult](react.md#usepaginatedqueryresult) que incluye los elementos cargados
actualmente, el estado de la paginación y una función `loadMore`.

#### Definido en \{#defined-in\}

[react/use&#95;paginated&#95;query2.ts:72](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query2.ts#L72)

***

### useQueries \{#usequeries\}

▸ **useQueries**(`queries`): `Record`&lt;`string`, `any` | `undefined` | `Error`&gt;

Carga un número variable de consultas reactivas de Convex.

`useQueries` es similar a [useQuery](react.md#usequery), pero permite
cargar múltiples consultas, lo cual puede ser útil para cargar un número dinámico
de consultas sin violar las reglas de los hooks de React.

Este hook acepta un objeto cuyas claves son identificadores para cada consulta y cuyos
valores son objetos de `{ query: FunctionReference, args: Record<string, Value> }`. La
`query` es una FunctionReference de la función de consulta de Convex que se va a cargar, y los `args` son
los argumentos de esa función.

El hook devuelve un objeto que asigna cada identificador al resultado de la consulta,
`undefined` si la consulta aún se está cargando, o una instancia de `Error` si la consulta
arrojó una excepción.

Por ejemplo, si cargas una consulta como:

```typescript
const results = useQueries({
  messagesInGeneral: {
    query: "listMessages",
    args: { channel: "#general" }
  }
});
```

entonces el resultado sería:

```typescript
{
  messagesInGeneral: [{
    channel: "#general",
    body: "hello"
    _id: ...,
    _creationTime: ...
  }]
}
```

Este hook de React contiene estado interno que hará que se vuelva a renderizar
cada vez que cambie cualquiera de los resultados de la consulta.

Lanza un error si no se usa dentro de [ConvexProvider](react.md#convexprovider).

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `queries` | [`RequestForQueries`](react.md#requestforqueries) | Un objeto que asigna identificadores a objetos de `{query: string, args: Record<string, Value> }` que describen qué funciones de consulta se deben recuperar. |

#### Devuelve \{#returns\}

`Record`&lt;`string`, `any` | `undefined` | `Error`&gt;

Un objeto con las mismas claves que la entrada. Los valores son el resultado
de la función de consulta, `undefined` si todavía se está cargando, o un `Error` si
se lanzó una excepción.

#### Definido en \{#defined-in\}

[react/use&#95;queries.ts:61](https://github.com/get-convex/convex-js/blob/main/src/react/use_queries.ts#L61)