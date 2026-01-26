---
id: "react"
title: "模块：react"
custom_edit_url: null
---

将 Convex 集成到 React 应用的工具。

该模块包含：

1. [ConvexReactClient](../classes/react.ConvexReactClient.md)，一个用于在 React 中使用 Convex 的客户端。
2. [ConvexProvider](react.md#convexprovider)，一个将该客户端存储在 React 上下文中的组件。
3. [Authenticated](react.md#authenticated)、[Unauthenticated](react.md#unauthenticated) 和 [AuthLoading](react.md#authloading) 三个辅助身份验证组件。
4. React 钩子 [useQuery](react.md#usequery)、[useMutation](react.md#usemutation)、[useAction](react.md#useaction) 等，用于在 React 组件中访问该客户端。

## 用法 \{#usage\}

### 创建客户端实例 \{#creating-the-client\}

```typescript
import { ConvexReactClient } from "convex/react";

// 通常从环境变量中加载
const address = "https://small-mouse-123.convex.cloud"
const convex = new ConvexReactClient(address);
```

### 在 React 的 Context 中保存客户端 \{#storing-the-client-in-react-context\}

```typescript
import { ConvexProvider } from "convex/react";

<ConvexProvider client={convex}>
  <App />
</ConvexProvider>
```

### 使用 Auth 辅助方法 \{#using-the-auth-helpers\}

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

### 使用 React Hook \{#using-react-hooks\}

```typescript
import { useQuery, useMutation } from "convex/react";
import { api } from "../convex/_generated/api";

function App() {
  const counter = useQuery(api.getCounter.default);
  const increment = useMutation(api.incrementCounter.default);
  // 在此处编写你的组件!
}
```

## 类 \{#classes\}

* [ConvexReactClient](../classes/react.ConvexReactClient.md)

## 接口 \{#interfaces\}

* [ReactMutation](../interfaces/react.ReactMutation.md)
* [ReactAction](../interfaces/react.ReactAction.md)
* [Watch](../interfaces/react.Watch.md)
* [WatchQueryOptions](../interfaces/react.WatchQueryOptions.md)
* [MutationOptions](../interfaces/react.MutationOptions.md)
* [ConvexReactClientOptions](../interfaces/react.ConvexReactClientOptions.md)

## 参考 \{#references\}

### AuthTokenFetcher \{#authtokenfetcher\}

重新导出 [AuthTokenFetcher](browser.md#authtokenfetcher)

## 类型别名 \{#type-aliases\}

### ConvexAuthState \{#convexauthstate\}

Ƭ **ConvexAuthState**: `Object`

用于表示 Convex 身份验证集成状态的类型。

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 |
| :------ | :------ |
| `isLoading` | `boolean` |
| `isAuthenticated` | `boolean` |

#### 定义于 \{#defined-in\}

[react/ConvexAuthState.tsx:26](https://github.com/get-convex/convex-js/blob/main/src/react/ConvexAuthState.tsx#L26)

***

### OptionalRestArgsOrSkip \{#optionalrestargsorskip\}

Ƭ **OptionalRestArgsOrSkip**&lt;`FuncRef`&gt;: `FuncRef`[`"_args"`] extends `EmptyObject` ? [args?: EmptyObject | &quot;skip&quot;] : [args: FuncRef[&quot;&#95;args&quot;] | &quot;skip&quot;]

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `FuncRef` | 受限为 [`FunctionReference`](server.md#functionreference)&lt;`any`&gt; 的子类型 |

#### 定义于 \{#defined-in\}

[react/client.ts:799](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L799)

***

### Preloaded \{#preloaded\}

Ƭ **Preloaded**&lt;`Query`&gt;: `Object`

预加载的查询载荷，应传递给客户端组件，
并再传递给 [usePreloadedQuery](react.md#usepreloadedquery)。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 |
| :------ | :------ |
| `__type` | `Query` |
| `_name` | `string` |
| `_argsJSON` | `string` |
| `_valueJSON` | `string` |

#### 定义在 \{#defined-in\}

[react/hydration.tsx:12](https://github.com/get-convex/convex-js/blob/main/src/react/hydration.tsx#L12)

***

### PaginatedQueryReference \{#paginatedqueryreference\}

Ƭ **PaginatedQueryReference**: [`FunctionReference`](server.md#functionreference)&lt;`"query"`, `"public"`, &#123; `paginationOpts`: [`PaginationOptions`](../interfaces/server.PaginationOptions.md)  &#125;, [`PaginationResult`](../interfaces/server.PaginationResult.md)&lt;`any`&gt;&gt;

一个可与 [usePaginatedQuery](react.md#usepaginatedquery) 配合使用的 [FunctionReference](server.md#functionreference)。

该函数引用必须：

* 指向一个 public 查询
* 具有名为 &quot;paginationOpts&quot; 且类型为 [PaginationOptions](../interfaces/server.PaginationOptions.md) 的参数
* 具有返回类型 [PaginationResult](../interfaces/server.PaginationResult.md)。

#### 定义于 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:31](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L31)

***

### UsePaginatedQueryResult \{#usepaginatedqueryresult\}

Ƭ **UsePaginatedQueryResult**&lt;`Item`&gt;: &#123; `results`: `Item`[] ; `loadMore`: (`numItems`: `number`) =&gt; `void`  &#125; &amp; &#123; `status`: `"LoadingFirstPage"` ; `isLoading`: `true`  &#125; | &#123; `status`: `"CanLoadMore"` ; `isLoading`: `false`  &#125; | &#123; `status`: `"LoadingMore"` ; `isLoading`: `true`  &#125; | &#123; `status`: `"Exhausted"` ; `isLoading`: `false`  &#125;

调用 [usePaginatedQuery](react.md#usepaginatedquery) hook 的结果。

包括：

* `results` - 当前已加载结果的数组。
* `isLoading` - 该 hook 当前是否正在加载结果。
* `status` - 分页的状态。可能的状态为：
  * &quot;LoadingFirstPage&quot;: 该 hook 正在加载结果的第一页。
  * &quot;CanLoadMore&quot;: 此查询可能还有更多项可获取。调用 `loadMore` 来获取下一页。
  * &quot;LoadingMore&quot;: 当前正在加载另一页结果。
  * &quot;Exhausted&quot;: 已经分页到列表末尾。
* `loadMore(n)` - 用于获取更多结果的回调。只有当 `status` 为 &quot;CanLoadMore&quot; 时才会获取更多结果。

#### 类型参数 \{#type-parameters\}

| 名称 |
| :------ |
| `Item` |

#### 定义于 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:479](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L479)

***

### PaginationStatus \{#paginationstatus\}

Ƭ **PaginationStatus**: [`UsePaginatedQueryResult`](react.md#usepaginatedqueryresult)&lt;`any`&gt;[`"status"`]

[UsePaginatedQueryResult](react.md#usepaginatedqueryresult) 中可能的分页状态。

这是由字符串字面量组成的联合类型。

#### 定义于 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:507](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L507)

***

### PaginatedQueryArgs \{#paginatedqueryargs\}

Ƭ **PaginatedQueryArgs**&lt;`Query`&gt;: [`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt;

给定一个 [PaginatedQueryReference](react.md#paginatedqueryreference)，获取该查询的参数对象的类型，但不包括 `paginationOpts` 参数。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### 定义于 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:515](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L515)

***

### PaginatedQueryItem \{#paginatedqueryitem\}

Ƭ **PaginatedQueryItem**&lt;`Query`&gt;: [`FunctionReturnType`](server.md#functionreturntype)&lt;`Query`&gt;[`"page"`][`number`]

给定一个 [PaginatedQueryReference](react.md#paginatedqueryreference)，获取分页结果中单个条目的类型。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | 需继承自 [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### 定义于 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:524](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L524)

***

### UsePaginatedQueryReturnType \{#usepaginatedqueryreturntype\}

Ƭ **UsePaginatedQueryReturnType**&lt;`Query`&gt;: [`UsePaginatedQueryResult`](react.md#usepaginatedqueryresult)&lt;[`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt;&gt;

[usePaginatedQuery](react.md#usepaginatedquery) 的返回类型。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | 继承自 [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### 定义于 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:532](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L532)

***

### RequestForQueries \{#requestforqueries\}

Ƭ **RequestForQueries**: `Record`&lt;`string`, &#123; `query`: [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; ; `args`: `Record`&lt;`string`, [`Value`](values.md#value)&gt;  &#125;&gt;

一个表示用于加载多个查询的请求的对象。

该对象的键是标识符，值是包含查询函数以及要传递给该函数的参数的对象。

它用作 [useQueries](react.md#usequeries) 的参数。

#### 定义于 \{#defined-in\}

[react/use&#95;queries.ts:137](https://github.com/get-convex/convex-js/blob/main/src/react/use_queries.ts#L137)

## 函数 \{#functions\}

### useConvexAuth \{#useconvexauth\}

▸ **useConvexAuth**(): `Object`

在 React 组件中获取 [ConvexAuthState](react.md#convexauthstate)。

这依赖于在 React 组件树的上层存在一个 Convex 认证集成 provider。

#### 返回值 \{#returns\}

`Object`

当前的 [ConvexAuthState](react.md#convexauthstate) 状态。

| 名称 | 类型 |
| :------ | :------ |
| `isLoading` | `boolean` |
| `isAuthenticated` | `boolean` |

#### 定义于 \{#defined-in\}

[react/ConvexAuthState.tsx:43](https://github.com/get-convex/convex-js/blob/main/src/react/ConvexAuthState.tsx#L43)

***

### ConvexProviderWithAuth \{#convexproviderwithauth\}

▸ **ConvexProviderWithAuth**(`«destructured»`): `Element`

[ConvexProvider](react.md#convexprovider) 的替代版本，同时向该组件的后代提供
[ConvexAuthState](react.md#convexauthstate)。

使用它可以将任意认证提供商集成到 Convex 中。`useAuth` prop
应当是一个 React Hook，用于返回提供商的身份验证状态，
以及一个用于获取 JWT 访问令牌的函数。

如果 `useAuth` prop 对应的函数发生更新并导致组件重新渲染，那么认证状态
将切换为加载中状态，并且会再次调用 `fetchAccessToken()` 函数。

更多信息参见 [Custom Auth Integration](https://docs.convex.dev/auth/advanced/custom-auth)。

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children?` | `ReactNode` |
| › `client` | `IConvexReactClient` |
| › `useAuth` | () =&gt; &#123; `isLoading`: `boolean` ; `isAuthenticated`: `boolean` ; `fetchAccessToken`: (`args`: &#123; `forceRefreshToken`: `boolean`  &#125;) =&gt; `Promise`&lt;`null` | `string`&gt;  &#125; |

#### 返回值 \{#returns\}

`Element`

#### 定义于 \{#defined-in\}

[react/ConvexAuthState.tsx:75](https://github.com/get-convex/convex-js/blob/main/src/react/ConvexAuthState.tsx#L75)

***

### Authenticated \{#authenticated\}

▸ **Authenticated**(`«destructured»`): `null` | `Element`

当客户端已通过身份验证时渲染其子元素。

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children` | `ReactNode` |

#### 返回值 \{#returns\}

`null` | `Element`

#### 定义于 \{#defined-in\}

[react/auth&#95;helpers.tsx:10](https://github.com/get-convex/convex-js/blob/main/src/react/auth_helpers.tsx#L10)

***

### Unauthenticated \{#unauthenticated\}

▸ **Unauthenticated**(`«destructured»`): `null` | `Element`

如果客户端已启用身份验证但当前用户未通过验证，则渲染其子元素。

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children` | `ReactNode` |

#### 返回 \{#returns\}

`null` | `Element`

#### 定义于 \{#defined-in\}

[react/auth&#95;helpers.tsx:23](https://github.com/get-convex/convex-js/blob/main/src/react/auth_helpers.tsx#L23)

***

### AuthLoading \{#authloading\}

▸ **AuthLoading**(`«destructured»`): `null` | `Element`

如果客户端未使用身份验证，或当前正在进行身份验证，则渲染其子组件。

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children` | `ReactNode` |

#### 返回值 \{#returns\}

`null` | `Element`

#### 定义于 \{#defined-in\}

[react/auth&#95;helpers.tsx:37](https://github.com/get-convex/convex-js/blob/main/src/react/auth_helpers.tsx#L37)

***

### useConvex \{#useconvex\}

▸ **useConvex**(): [`ConvexReactClient`](../classes/react.ConvexReactClient.md)

在 React 组件中获取 [ConvexReactClient](../classes/react.ConvexReactClient.md)。

这要求在 React 组件树的上层有一个 [ConvexProvider](react.md#convexprovider)。

#### 返回值 \{#returns\}

[`ConvexReactClient`](../classes/react.ConvexReactClient.md)

当前活动的 [ConvexReactClient](../classes/react.ConvexReactClient.md) 对象，或 `undefined`。

#### 定义于 \{#defined-in\}

[react/client.ts:774](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L774)

***

### ConvexProvider \{#convexprovider\}

▸ **ConvexProvider**(`props`, `deprecatedLegacyContext?`): `null` | `ReactElement`&lt;`any`, `any`&gt;

为该组件的后代组件提供一个活动的 Convex [ConvexReactClient](../classes/react.ConvexReactClient.md)。

将你的应用包裹在该组件外层，以便使用 Convex 的 `useQuery`、
`useMutation` 和 `useConvex` Hook。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `props` | `Object` | 一个包含 `client` 属性的对象，该属性引用一个 [ConvexReactClient](../classes/react.ConvexReactClient.md)。 |
| `props.client` | [`ConvexReactClient`](../classes/react.ConvexReactClient.md) | - |
| `props.children?` | `ReactNode` | - |
| `deprecatedLegacyContext?` | `any` | **`已弃用`** **`参见`** [React 文档](https://legacy.reactjs.org/docs/legacy-context.html#referencing-context-in-lifecycle-methods) |

#### 返回 \{#returns\}

`null` | `ReactElement`&lt;`any`, `any`&gt;

#### 定义于 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/@types+react@18.3.26/node&#95;modules/@types/react/ts5.0/index.d.ts:1129

***

### useQuery \{#usequery\}

▸ **useQuery**&lt;`Query`&gt;(`query`, `...args`): `Query`[`"_returnType"`] | `undefined`

在 React 组件中加载一个响应式查询。

这个 React hook 内部维护状态，当查询结果发生变化时会触发重新渲染。

如果没有在 [ConvexProvider](react.md#convexprovider) 下使用，则会抛出错误。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `query` | `Query` | 要执行的公共查询的 [FunctionReference](server.md#functionreference)，例如 `api.dir1.dir2.filename.func`。 |
| `...args` | [`OptionalRestArgsOrSkip`](react.md#optionalrestargsorskip)&lt;`Query`&gt; | 传给查询函数的参数；如果不应加载该查询，则为字符串 &quot;skip&quot;。 |

#### 返回值 \{#returns\}

`Query`[`"_returnType"`] | `undefined`

查询结果。如果查询仍在加载中，则返回 `undefined`。

#### 定义于 \{#defined-in\}

[react/client.ts:820](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L820)

***

### useMutation \{#usemutation\}

▸ **useMutation**&lt;`Mutation`&gt;(`mutation`): [`ReactMutation`](../interfaces/react.ReactMutation.md)&lt;`Mutation`&gt;

构造一个新的 [ReactMutation](../interfaces/react.ReactMutation.md)。

`Mutation` 对象可以像函数一样被调用，以请求执行对应的 Convex 函数，或者配合
[optimistic updates](https://docs.convex.dev/using/optimistic-updates) 做进一步配置。

该 hook 返回的值在多次渲染之间是稳定的，因此可以安全地用于 React 的依赖数组以及依赖对象标识的记忆化逻辑中，而不会导致重新渲染。

如果不在 [ConvexProvider](react.md#convexprovider) 之内使用会抛出错误。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](server.md#functionreference)&lt;`"mutation"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | 要运行的公开变更函数的 [FunctionReference](server.md#functionreference)，例如 `api.dir1.dir2.filename.func`。 |

#### 返回值 \{#returns\}

[`ReactMutation`](../interfaces/react.ReactMutation.md)&lt;`Mutation`&gt;

具有该名称的 [ReactMutation](../interfaces/react.ReactMutation.md) 对象。

#### 定义于 \{#defined-in\}

[react/client.ts:872](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L872)

***

### useAction \{#useaction\}

▸ **useAction**&lt;`Action`&gt;(`action`): [`ReactAction`](../interfaces/react.ReactAction.md)&lt;`Action`&gt;

创建一个新的 [ReactAction](../interfaces/react.ReactAction.md)。

`Action` 对象可以像函数一样被调用，用于请求执行对应的 Convex 函数。

此 hook 返回的值在多次渲染之间保持稳定，因此可以用于 React 依赖数组以及依赖对象标识的 memoization 逻辑，而不会导致组件重新渲染。

如果不在 [ConvexProvider](react.md#convexprovider) 下使用，则会抛出错误。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](server.md#functionreference)&lt;`"action"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `action` | `Action` | 要运行的公开操作的 [FunctionReference](server.md#functionreference)，例如 `api.dir1.dir2.filename.func`。 |

#### 返回值 \{#returns\}

[`ReactAction`](../interfaces/react.ReactAction.md)&lt;`Action`&gt;

名为该名称的 [ReactAction](../interfaces/react.ReactAction.md) 对象。

#### 定义于 \{#defined-in\}

[react/client.ts:913](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L913)

***

### useConvexConnectionState \{#useconvexconnectionstate\}

▸ **useConvexConnectionState**(): [`ConnectionState`](browser.md#connectionstate)

用于获取当前 [ConnectionState](browser.md#connectionstate) 并订阅其变化的 React Hook。

此 Hook 会返回当前连接状态，并在连接状态的任意部分发生变化时（例如在线/离线切换、请求开始/完成等）自动重新渲染。

ConnectionState 的结构将来可能会发生变化，这可能会导致此 Hook 更频繁地重新渲染。

如果未在 [ConvexProvider](react.md#convexprovider) 内使用，则会抛出错误。

#### 返回值 \{#returns\}

[`ConnectionState`](browser.md#connectionstate)

当前与 Convex 后端的 [ConnectionState](browser.md#connectionstate)。

#### 定义于 \{#defined-in\}

[react/client.ts:952](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L952)

***

### usePreloadedQuery \{#usepreloadedquery\}

▸ **usePreloadedQuery**&lt;`Query`&gt;(`preloadedQuery`): `Query`[`"_returnType"`]

在 React 组件中使用由 Server Component 通过 [preloadQuery](nextjs.md#preloadquery) 返回的 `Preloaded` 载荷来加载一个响应式查询。

这个 React hook 包含内部状态，当查询结果发生变化时会触发组件重新渲染。

如果没有在 [ConvexProvider](react.md#convexprovider) 下使用，则会抛出错误。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `preloadedQuery` | [`Preloaded`](react.md#preloaded)&lt;`Query`&gt; | 来自 Server Component 的 `Preloaded` 查询数据。 |

#### 返回值 \{#returns\}

`Query`[`"_returnType"`]

查询结果。起初返回由 Server Component 获取的结果，此后返回由客户端获取的结果。

#### 定义于 \{#defined-in\}

[react/hydration.tsx:34](https://github.com/get-convex/convex-js/blob/main/src/react/hydration.tsx#L34)

***

### usePaginatedQuery \{#usepaginatedquery\}

▸ **usePaginatedQuery**&lt;`Query`&gt;(`query`, `args`, `options`): [`UsePaginatedQueryReturnType`](react.md#usepaginatedqueryreturntype)&lt;`Query`&gt;

以响应式方式从分页查询中加载数据，用于构建一个不断增长的列表。

这可以用来实现「无限滚动」UI。

这个 hook 必须与符合
[PaginatedQueryReference](react.md#paginatedqueryreference) 的公开查询引用一起使用。

`usePaginatedQuery` 会将所有结果页拼接成一个列表，
并在请求更多项目时管理 continuation 游标。

示例用法：

```typescript
const { results, status, isLoading, loadMore } = usePaginatedQuery(
  api.messages.list,
  { channel: "#general" },
  { initialNumItems: 5 }
);
```

如果查询引用或参数发生变化，分页状态将被重置为第一页。类似地，如果任意一页出现 `InvalidCursor`
错误或与数据量过大相关的错误，分页状态也会被重置为第一页。

要进一步了解分页，请参阅[分页查询](https://docs.convex.dev/database/pagination)。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | 扩展自 [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `query` | `Query` | 指向要运行的公共查询函数的 FunctionReference。 |
| `args` | `"skip"` | [`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt; | 查询函数的参数对象，但不包括 `paginationOpts` 属性。该属性由此 hook 注入。 |
| `options` | `Object` | 一个对象，用于指定在第一页中要加载的 `initialNumItems`。 |
| `options.initialNumItems` | `number` | - |

#### 返回值 \{#returns\}

[`UsePaginatedQueryReturnType`](react.md#usepaginatedqueryreturntype)&lt;`Query`&gt;

一个 [UsePaginatedQueryResult](react.md#usepaginatedqueryresult)，其中包含当前已加载的
条目、分页状态，以及一个 `loadMore` 函数。

#### 定义于 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:162](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L162)

***

### resetPaginationId \{#resetpaginationid\}

▸ **resetPaginationId**(): `void`

仅用于测试时重置分页 ID，方便测试了解当前的分页 ID。

#### 返回类型 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:458](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L458)

***

### optimisticallyUpdateValueInPaginatedQuery \{#optimisticallyupdatevalueinpaginatedquery\}

▸ **optimisticallyUpdateValueInPaginatedQuery**&lt;`Query`&gt;(`localStore`, `query`, `args`, `updateValue`): `void`

乐观地更新分页列表中的值。

此乐观更新用于配合
[usePaginatedQuery](react.md#usepaginatedquery) 更新通过其加载的数据。它会在所有已加载的分页中，将 `updateValue` 应用于列表中的每个元素来更新列表。

这仅会应用于名称和参数都匹配的查询。

示例用法：

```ts
const myMutation = useMutation(api.myModule.myMutation)
.withOptimisticUpdate((localStore, mutationArg) => {

  // 乐观更新 ID 为 `mutationArg` 的文档,
  // 为其添加一个额外的属性。

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

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | 继承自 [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `localStore` | [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md) | 要更新的 [OptimisticLocalStore](../interfaces/browser.OptimisticLocalStore.md)。 |
| `query` | `Query` | 要更新的分页查询的 [FunctionReference](server.md#functionreference)。 |
| `args` | [`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt; | 传递给查询函数的参数对象，不包含 `paginationOpts` 属性。 |
| `updateValue` | (`currentValue`: [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt;) =&gt; [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt; | 用于生成新值的函数。 |

#### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:578](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L578)

***

### insertAtTop \{#insertattop\}

▸ **insertAtTop**&lt;`Query`&gt;(`options`): `void`

更新分页查询，在列表顶部插入一个元素。

无论当前的排序方式如何，如果列表是降序，
插入的元素会被视为“最大”的元素；如果是升序，
则会被视为“最小”的元素。

示例：

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

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | 扩展自 [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `options` | `Object` | - |
| `options.paginatedQuery` | `Query` | 指向分页查询的函数引用。 |
| `options.argsToMatch?` | `Partial`&lt;[`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt;&gt; | 必须在每个相关分页查询中出现的可选参数。如果你使用同一个查询函数配合不同参数来加载不同的列表，这会很有用。 |
| `options.localQueryStore` | [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md) |  |
| `options.item` | [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt; | 要插入的条目。 |

#### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:640](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L640)

***

### insertAtBottomIfLoaded \{#insertatbottomifloaded\}

▸ **insertAtBottomIfLoaded**&lt;`Query`&gt;(`options`): `void`

更新分页查询，使其在列表底部插入一个元素。

无论排序顺序如何：如果列表是降序，插入的元素会被视为“最小”的元素；如果是升序，则会被视为“最大”的元素。

只有在最后一页已加载的情况下这才会生效，否则会导致元素被插入到当前已加载内容的末尾（也就是列表中间），并且在乐观更新结束后被移除。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | 扩展自 [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### 参数 \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `options` | `Object` | - |
| `options.paginatedQuery` | `Query` | 指向分页查询的函数引用。 |
| `options.argsToMatch?` | `Partial`&lt;[`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt;&gt; | 可选参数，这些参数必须包含在每个相关的分页查询中。如果你对同一个查询函数使用不同的参数来加载不同的列表，这会很有用。 |
| `options.localQueryStore` | [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md) |  |
| `options.item` | [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt; | - |

#### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:689](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L689)

***

### insertAtPosition \{#insertatposition\}

▸ **insertAtPosition**&lt;`Query`&gt;(`options`): `void`

这是一个辅助函数，用于在分页查询中的指定位置插入一项。

你必须提供 sortOrder，以及一个函数，用于根据列表中的一项计算排序键（值数组）。

仅当服务端查询使用与乐观更新相同的排序顺序和排序键时，此方法才有效。

示例：

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

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | 扩展自 [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### 参数 \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `options` | `Object` | - |
| `options.paginatedQuery` | `Query` | 指向分页查询的函数引用。 |
| `options.argsToMatch?` | `Partial`&lt;[`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt;&gt; | 在每个相关分页查询中都必须包含、且需要匹配的可选参数。如果你对同一个查询函数使用不同的参数来加载不同的列表，这会很有用。 |
| `options.sortOrder` | `"asc"` | `"desc"` | 分页查询的排序方向（&quot;asc&quot; 或 &quot;desc&quot;）。 |
| `options.sortKeyFromItem` | (`element`: [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt;) =&gt; [`Value`](values.md#value) | [`Value`](values.md#value)[] | 一个用于从列表元素派生排序键（值数组）的函数。推荐包含 `_creationTime` 之类用于消除并列的字段。 |
| `options.localQueryStore` | [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md) |  |
| `options.item` | [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt; | 要插入的项。 |

#### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:770](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L770)

***

### usePaginatedQuery_experimental \{#usepaginatedquery_experimental\}

▸ **usePaginatedQuery&#95;experimental**&lt;`Query`&gt;(`query`, `args`, `options`): [`UsePaginatedQueryReturnType`](react.md#usepaginatedqueryreturntype)&lt;`Query`&gt;

全新的实验性 usePaginatedQuery 实现，将在未来替换当前版本。

以响应式方式从分页查询中加载数据，用于创建一个不断增长的列表。

这是一个依赖全新客户端分页逻辑的替代实现。

它可用于驱动“无限滚动”（infinite scroll）UI。

此 hook 必须与符合
[PaginatedQueryReference](react.md#paginatedqueryreference)
的公共查询引用一起使用。

`usePaginatedQuery` 会将所有结果页拼接成一个单一列表，
并在请求更多条目时管理续传游标（continuation cursor）。

示例用法：

```typescript
const { results, status, isLoading, loadMore } = usePaginatedQuery(
  api.messages.list,
  { channel: "#general" },
  { initialNumItems: 5 }
);
```

如果查询引用或参数发生变化，分页状态会被重置为第一页。类似地，如果任何一页返回 `InvalidCursor`
错误，或者出现与数据量过大相关的错误，分页状态也会被重置为第一页。

要进一步了解分页，请参阅[分页查询](https://docs.convex.dev/database/pagination)。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | 扩展自 [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `query` | `Query` | 指向要运行的公共查询函数的 FunctionReference。 |
| `args` | `"skip"` | [`PaginatedQueryArgs`](react.md#paginatedqueryargs)&lt;`Query`&gt; | 查询函数的参数对象，不包括 `paginationOpts` 属性。该属性由此 hook 注入。 |
| `options` | `Object` | 一个对象，用于指定在第一页中要加载的 `initialNumItems`。 |
| `options.initialNumItems` | `number` | - |

#### 返回值 \{#returns\}

[`UsePaginatedQueryReturnType`](react.md#usepaginatedqueryreturntype)&lt;`Query`&gt;

一个 [UsePaginatedQueryResult](react.md#usepaginatedqueryresult)，其中包含当前已加载的
项目、分页状态，以及一个 `loadMore` 函数。

#### 定义于 \{#defined-in\}

[react/use&#95;paginated&#95;query2.ts:72](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query2.ts#L72)

***

### useQueries \{#usequeries\}

▸ **useQueries**(`queries`): `Record`&lt;`string`, `any` | `undefined` | `Error`&gt;

加载数量可变的响应式 Convex 查询。

`useQueries` 类似于 [useQuery](react.md#usequery)，但它允许
一次加载多个查询，这在需要加载动态数量的查询时很有用，
并且不会违反 React hooks 的规则。

这个 hook 接收一个对象，其中键是每个查询的标识符，
值是形如 `{ query: FunctionReference, args: Record<string, Value> }` 的对象。
`query` 是要加载的 Convex 查询函数的 FunctionReference，
`args` 则是传给该函数的参数。

该 hook 返回一个对象，将每个标识符映射到对应查询的结果；
如果查询仍在加载中则为 `undefined`，如果查询抛出异常则为一个 `Error` 实例。

例如，如果你这样加载一个查询：

```typescript
const results = useQueries({
  messagesInGeneral: {
    query: "listMessages",
    args: { channel: "#general" }
  }
});
```

那么结果如下：

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

这个 React Hook 包含内部状态，当任一查询结果发生变化时都会触发重新渲染。

如果不在 [ConvexProvider](react.md#convexprovider) 下使用，则会抛出错误。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `queries` | [`RequestForQueries`](react.md#requestforqueries) | 一个对象，它将标识符映射到形如 `{query: string, args: Record&lt;string, Value&gt; }` 的对象，用于描述要获取哪些查询函数。 |

#### 返回值 \{#returns\}

`Record`&lt;`string`, `any` | `undefined` | `Error`&gt;

一个键与输入相同的对象。其值要么是查询函数的结果，要么在仍在加载时为 `undefined`，要么在抛出异常时为 `Error`。

#### 定义于 \{#defined-in\}

[react/use&#95;queries.ts:61](https://github.com/get-convex/convex-js/blob/main/src/react/use_queries.ts#L61)