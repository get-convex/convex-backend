---
id: "nextjs"
title: "模块：nextjs"
custom_edit_url: null
---

用于在使用服务端渲染的 Next.js 应用中集成 Convex 的辅助函数。

该模块包含：

1. [preloadQuery](nextjs.md#preloadquery)，用于为响应式客户端组件预加载数据。
2. [fetchQuery](nextjs.md#fetchquery)、[fetchMutation](nextjs.md#fetchmutation) 和 [fetchAction](nextjs.md#fetchaction)，用于在 Next.js Server Components、Server Actions 和 Route Handlers 中加载 Convex 数据并执行变更和操作。

## 用法 \{#usage\}

所有导出的函数都假定已在 `NEXT_PUBLIC_CONVEX_URL` 环境变量中设置 Convex 部署 URL。`npx convex dev` 会在本地开发时自动为你完成该设置。

### 预加载数据 \{#preloading-data\}

在 Server Component 内预加载数据：

```typescript
import { preloadQuery } from "convex/nextjs";
import { api } from "@/convex/_generated/api";
import ClientComponent from "./ClientComponent";

export async function ServerComponent() {
  const preloaded = await preloadQuery(api.foo.baz);
  return <ClientComponent preloaded={preloaded} />;
}
```

并将它传递给一个 Client 组件：

```typescript
import { Preloaded, usePreloadedQuery } from "convex/react";
import { api } from "@/convex/_generated/api";

export function ClientComponent(props: {
  preloaded: Preloaded<typeof api.foo.baz>;
}) {
  const data = usePreloadedQuery(props.preloaded);
  // 渲染 `data`...
}
```

## 类型别名 \{#type-aliases\}

### NextjsOptions \{#nextjsoptions\}

Ƭ **NextjsOptions**: `Object`

传递给 [preloadQuery](nextjs.md#preloadquery)、[fetchQuery](nextjs.md#fetchquery)、[fetchMutation](nextjs.md#fetchmutation) 和 [fetchAction](nextjs.md#fetchaction) 的选项。

#### 类型声明 \{#type-declaration\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `token?` | `string` | 用于函数调用的 JWT 编码的 OpenID Connect 身份验证令牌。 |
| `url?` | `string` | 用于函数调用的 Convex 部署 URL。如果未提供，默认为 `process.env.NEXT_PUBLIC_CONVEX_URL`。在未来的版本中，如果在此显式传入 undefined（例如由于缺少环境变量），将会抛出错误。 |
| `skipConvexDeploymentUrlCheck?` | `boolean` | 跳过验证 Convex 部署 URL 是否形如 `https://happy-animal-123.convex.cloud` 或 localhost。如果运行的是使用不同 URL 的自托管 Convex 后端，这会很有用。默认值为 `false`。 |

#### 定义于 \{#defined-in\}

[nextjs/index.ts:60](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L60)

## 函数 \{#functions\}

### preloadQuery \{#preloadquery\}

▸ **preloadQuery**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`Preloaded`](react.md#preloaded)&lt;`Query`&gt;&gt;

执行一个 Convex 查询函数，并返回一个 `Preloaded`
载荷，可在 Client 组件中传递给 [usePreloadedQuery](react.md#usepreloadedquery)。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### 参数 \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | 要运行的公共查询的 [FunctionReference](server.md#functionreference)，形如 `api.dir1.dir2.filename.func`。 |
| `...args` | [`ArgsAndOptions`](server.md#argsandoptions)&lt;`Query`, [`NextjsOptions`](nextjs.md#nextjsoptions)&gt; | 查询的参数对象。如果省略，则默认为 `{}`。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`Preloaded`](react.md#preloaded)&lt;`Query`&gt;&gt;

一个会解析为 `Preloaded` 负载的 `Promise`。

#### 定义于 \{#defined-in\}

[nextjs/index.ts:101](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L101)

***

### preloadedQueryResult \{#preloadedqueryresult\}

▸ **preloadedQueryResult**&lt;`Query`&gt;(`preloaded`): [`FunctionReturnType`](server.md#functionreturntype)&lt;`Query`&gt;

返回通过 [preloadQuery](nextjs.md#preloadquery) 执行的查询结果。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `preloaded` | [`Preloaded`](react.md#preloaded)&lt;`Query`&gt; | 由 [preloadQuery](nextjs.md#preloadquery) 返回的 `Preloaded` 对象。 |

#### 返回值 \{#returns\}

[`FunctionReturnType`](server.md#functionreturntype)&lt;`Query`&gt;

查询结果。

#### 定义于 \{#defined-in\}

[nextjs/index.ts:120](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L120)

***

### fetchQuery \{#fetchquery\}

▸ **fetchQuery**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Query`&gt;&gt;

执行一个 Convex 查询函数。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### 参数 \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | 指向要运行的公共查询的 [FunctionReference](server.md#functionreference)，例如 `api.dir1.dir2.filename.func`。 |
| `...args` | [`ArgsAndOptions`](server.md#argsandoptions)&lt;`Query`, [`NextjsOptions`](nextjs.md#nextjsoptions)&gt; | 查询的参数对象。如果省略此参数，则参数将为 `{}`。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Query`&gt;&gt;

一个解析为该查询结果的 Promise。

#### 定义于 \{#defined-in\}

[nextjs/index.ts:136](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L136)

***

### fetchMutation \{#fetchmutation\}

▸ **fetchMutation**&lt;`Mutation`&gt;(`mutation`, `...args`): `Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

执行 Convex 的变更函数。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](server.md#functionreference)&lt;`"mutation"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | 要执行的公共变更函数的 [FunctionReference](server.md#functionreference)，例如 `api.dir1.dir2.filename.func`。 |
| `...args` | [`ArgsAndOptions`](server.md#argsandoptions)&lt;`Mutation`, [`NextjsOptions`](nextjs.md#nextjsoptions)&gt; | 该变更的参数对象。如果省略此项，参数将默认为 `{}`。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

一个将被解析为该变更结果的 Promise。

#### 定义于 \{#defined-in\}

[nextjs/index.ts:155](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L155)

***

### fetchAction \{#fetchaction\}

▸ **fetchAction**&lt;`Action`&gt;(`action`, `...args`): `Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Action`&gt;&gt;

执行一个 Convex 操作函数。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](server.md#functionreference)&lt;`"action"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `action` | `Action` | 要运行的公开操作的 [FunctionReference](server.md#functionreference)，例如 `api.dir1.dir2.filename.func`。 |
| `...args` | [`ArgsAndOptions`](server.md#argsandoptions)&lt;`Action`, [`NextjsOptions`](nextjs.md#nextjsoptions)&gt; | 传给该操作的参数对象。如果省略，则参数为 `{}`。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Action`&gt;&gt;

一个会解析为该操作结果的 Promise。

#### 定义于 \{#defined-in\}

[nextjs/index.ts:176](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L176)