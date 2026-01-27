---
id: "server.HttpRouter"
title: "类：HttpRouter"
custom_edit_url: null
---

[server](../modules/server.md).HttpRouter

用于为 [httpActionGeneric](../modules/server.md#httpactiongeneric) 指定路径和方法的 HTTP 路由器

一个示例 `convex/http.js` 文件可能如下所示。

```js
import { httpRouter } from "convex/server";
import { getMessagesByAuthor } from "./getMessagesByAuthor";
import { httpAction } from "./_generated/server";

const http = httpRouter();

// HTTP 操作函数可以内联定义...
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

// ...也可以从其他文件导入。
http.route({
  path: "/getMessagesByAuthor",
  method: "GET",
  handler: getMessagesByAuthor,
});

// Convex 要求路由器作为 `convex/http.js` 的默认导出。
export default http;
```

## 构造函数 \{#constructors\}

### 构造函数 \{#constructor\}

• **new HttpRouter**()

## 属性 \{#properties\}

### exactRoutes \{#exactroutes\}

• **exactRoutes**: `Map`&lt;`string`, `Map`&lt;`"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, [`PublicHttpAction`](../modules/server.md#publichttpaction)&gt;&gt;

#### 定义于 \{#defined-in\}

[server/router.ts:143](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L143)

***

### prefixRoutes \{#prefixroutes\}

• **prefixRoutes**: `Map`&lt;`"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, `Map`&lt;`string`, [`PublicHttpAction`](../modules/server.md#publichttpaction)&gt;&gt;

#### 定义于 \{#defined-in\}

[server/router.ts:144](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L144)

***

### isRouter \{#isrouter\}

• **isRouter**: `true`

#### 定义于 \{#defined-in\}

[server/router.ts:145](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L145)

## 方法 \{#methods\}

### route \{#route\}

▸ **route**(`spec`): `void`

指定一个 HttpAction，用于响应特定 HTTP 方法（例如 “GET”）以及某个路径或路径前缀的请求。

路径必须以斜杠开头。路径前缀也必须以斜杠结尾。

```js
// matches `/profile` (but not `/profile/`)
http.route({ path: "/profile", method: "GET", handler: getProfile})

// 匹配 `/profiles/`、`/profiles/abc` 和 `/profiles/a/c/b`(但不匹配 `/profile`)
http.route({ pathPrefix: "/profile/", method: "GET", handler: getProfile})
```

#### 参数 \{#parameters\}

| 参数名 | 类型 |
| :------ | :------ |
| `spec` | [`RouteSpec`](../modules/server.md#routespec) |

#### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[server/router.ts:161](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L161)

***

### getRoutes \{#getroutes\}

▸ **getRoutes**(): readonly [`string`, `"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, [`PublicHttpAction`](../modules/server.md#publichttpaction)][]

返回一个已配置路由的 HTTP 操作函数列表。

这些结果用于填充 Convex 仪表盘中 Functions 页面显示的路由列表。

#### 返回值 \{#returns\}

readonly [`string`, `"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, [`PublicHttpAction`](../modules/server.md#publichttpaction)][]

* 一个由 [path, method, endpoint] 元组构成的数组。

#### 定义于 \{#defined-in\}

[server/router.ts:229](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L229)

***

### lookup \{#lookup\}

▸ **lookup**(`path`, `method`): `null` | readonly [[`PublicHttpAction`](../modules/server.md#publichttpaction), `"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, `string`]

返回对应的 HTTP 操作，以及其路由后的请求路径和方法。

返回的路径和方法用于日志记录和指标监控，并且应当
与 `getRoutes` 返回的某个路由匹配。

例如，

```js
http.route({ pathPrefix: "/profile/", method: "GET", handler: getProfile});

http.lookup("/profile/abc", "GET") // 返回 [getProfile, "GET", "/profile/*"]
```

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `path` | `string` |
| `method` | `"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"` | `"HEAD"` |

#### 返回值 \{#returns\}

`null` | readonly [[`PublicHttpAction`](../modules/server.md#publichttpaction), `"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, `string`]

* 一个元组 [[PublicHttpAction](../modules/server.md#publichttpaction), HTTP 方法, 路径] 或 null。

#### 定义于 \{#defined-in\}

[server/router.ts:275](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L275)

***

### runRequest \{#runrequest\}

▸ **runRequest**(`argsStr`, `requestRoute`): `Promise`&lt;`string`&gt;

给定一个 Request 对象的 JSON 字符串形式，通过对该请求进行路由并运行相应的 endpoint，或返回一个 404 响应，从而返回一个 Response。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `argsStr` | `string` | 表示一个 Request 对象的 JSON 字符串。 |
| `requestRoute` | `string` | - |

#### 返回值 \{#returns\}

`Promise`&lt;`string`&gt;

* 一个 Response 对象。

#### 定义于 \{#defined-in\}

[server/router.ts:304](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L304)