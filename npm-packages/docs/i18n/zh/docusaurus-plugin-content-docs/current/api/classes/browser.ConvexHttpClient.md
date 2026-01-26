---
id: "browser.ConvexHttpClient"
title: "类：ConvexHttpClient"
custom_edit_url: null
---

[browser](../modules/browser.md).ConvexHttpClient

一个通过 HTTP 运行查询和变更函数的 Convex 客户端。

此客户端是有状态的（它拥有用户凭据并会将变更排队执行），
因此在服务器中要注意避免在多个请求之间共享它。

这适用于服务端代码（例如 Netlify Lambdas）或非响应式
Web 应用。

## 构造函数 \{#constructors\}

### constructor \{#constructor\}

• **new ConvexHttpClient**(`address`, `options?`)

创建一个新的 [ConvexHttpClient](browser.ConvexHttpClient.md) 实例。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `address` | `string` | 你的 Convex 部署 URL，通常通过环境变量提供。例如：`https://small-mouse-123.convex.cloud`。 |
| `options?` | `Object` | 一个配置选项对象。- `skipConvexDeploymentUrlCheck` - 跳过验证 Convex 部署 URL 是否看起来像 `https://happy-animal-123.convex.cloud` 或 localhost。如果你运行的是使用不同 URL 的自托管 Convex 后端，这会很有用。- `logger` - 一个 logger 或布尔值。如果未提供，则将日志输出到控制台。你可以构造自己的 logger，将日志自定义为输出到其他位置或完全不输出日志，或者使用 `false` 作为空操作（no-op）logger 的简写。logger 是一个具有 4 个方法的对象：log()、warn()、error() 和 logVerbose()。这些方法可以像 console.log() 一样接收任意类型的多个参数。- `auth` - 一个包含可在 Convex 函数中访问的身份声明的 JWT。此身份可能会过期，因此可能需要稍后调用 `setAuth()`，但对于短生命周期的客户端，在这里指定该值会比较方便。- `fetch` - 一个自定义的 fetch 实现，用于该客户端发出的所有 HTTP 请求。 |
| `options.skipConvexDeploymentUrlCheck?` | `boolean` | - |
| `options.logger?` | `boolean` | `Logger` | - |
| `options.auth?` | `string` | - |
| `options.fetch?` | (`input`: `URL` | `RequestInfo`, `init?`: `RequestInit`) =&gt; `Promise`&lt;`Response`&gt;(`input`: `string` | `URL` | `Request`, `init?`: `RequestInit`) =&gt; `Promise`&lt;`Response`&gt; | - |

#### 定义于 \{#defined-in\}

[browser/http&#95;client.ts:97](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L97)

## 访问器 \{#accessors\}

### url \{#url\}

• `get` **url**(): `string`

返回该客户端的地址，可用于创建一个新的客户端实例。

不保证与创建该客户端时传入的地址完全一致；该地址可能已被规范化处理。

#### 返回值 \{#returns\}

`string`

#### 定义于 \{#defined-in\}

[browser/http&#95;client.ts:147](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L147)

## 方法 \{#methods\}

### backendUrl \{#backendurl\}

▸ **backendUrl**(): `string`

获取 [ConvexHttpClient](browser.ConvexHttpClient.md) 所连接后端的 URL。

**`Deprecated`**

请使用 `url`，它会返回末尾不带 `/api` 的 URL。

#### 返回值 \{#returns\}

`string`

指向 Convex 后端的 URL，其中包含客户端的 API 版本。

#### 定义于 \{#defined-in\}

[browser/http&#95;client.ts:137](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L137)

***

### setAuth \{#setauth\}

▸ **setAuth**(`value`): `void`

设置将用于后续查询和变更的身份验证令牌。

应在令牌发生变化时调用（例如因过期或刷新）。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `value` | `string` | 经过 JWT 编码的 OpenID Connect 身份令牌。 |

#### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[browser/http&#95;client.ts:158](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L158)

***

### clearAuth \{#clearauth\}

▸ **clearAuth**(): `void`

如果已设置，则清除当前的身份验证令牌。

#### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[browser/http&#95;client.ts:184](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L184)

***

### consistentQuery \{#consistentquery\}

▸ **consistentQuery**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

此 API 为实验性功能：其行为可能会更改或被移除。

在与此 HTTP 客户端运行的所有其他一致性查询具有相同时间戳的情况下，执行一个 Convex 查询函数。

对于长时间存在的 `ConvexHttpClient` 实例，这样做没有意义，因为 Convex
后端只能读取有限时间范围内的历史数据：早于 30 秒的历史数据可能无法获取。

请创建一个新的客户端以使用一致的时间点。

**`Deprecated`**

此 API 为实验性功能：其行为可能会更改或被移除。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `query` | `Query` | - |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; | 查询的参数对象。如果省略该参数，则默认为 `{}`。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

一个在查询完成后解析为该查询结果的 Promise。

#### 定义于 \{#defined-in\}

[browser/http&#95;client.ts:226](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L226)

***

### query \{#query\}

▸ **query**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

执行一个 Convex 查询函数。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `query` | `Query` | - |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; | 查询的参数对象。如果省略，该参数将默认为 `{}`。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

一个解析为查询结果的 Promise。

#### 定义于 \{#defined-in\}

[browser/http&#95;client.ts:270](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L270)

***

### 变更函数 \{#mutation\}

▸ **mutation**&lt;`Mutation`&gt;(`mutation`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

执行一个 Convex 变更函数。变更函数默认会被加入队列执行。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | - |
| `...args` | [`ArgsAndOptions`](../modules/server.md#argsandoptions)&lt;`Mutation`, [`HttpMutationOptions`](../modules/browser.md#httpmutationoptions)&gt; | 变更的参数对象。如果省略，则参数默认为 `{}`。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

一个会被解析为该变更结果的 Promise 对象。

#### 定义于 \{#defined-in\}

[browser/http&#95;client.ts:430](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L430)

***

### action \{#action\}

▸ **action**&lt;`Action`&gt;(`action`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

执行 Convex 操作函数。操作函数不会排队执行。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"action"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `action` | `Action` | - |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Action`&gt; | 操作的参数对象。如果省略，则参数对象默认为 `{}`。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

表示该操作结果的 `Promise`。

#### 定义于 \{#defined-in\}

[browser/http&#95;client.ts:453](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L453)