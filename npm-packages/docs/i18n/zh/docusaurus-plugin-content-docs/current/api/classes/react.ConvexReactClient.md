---
id: "react.ConvexReactClient"
title: "类：ConvexReactClient"
custom_edit_url: null
---

[react](../modules/react.md).ConvexReactClient

用于 React 中的 Convex 客户端。

它通过 WebSocket 加载响应式查询并执行变更函数。

## 构造函数 \{#constructors\}

### 构造函数 \{#constructor\}

• **new ConvexReactClient**(`address`, `options?`)

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `address` | `string` | 你的 Convex 部署的 URL，通常通过环境变量提供。例如：`https://small-mouse-123.convex.cloud`。 |
| `options?` | [`ConvexReactClientOptions`](../interfaces/react.ConvexReactClientOptions.md) | 完整说明请参阅 [ConvexReactClientOptions](../interfaces/react.ConvexReactClientOptions.md)。 |

#### 定义于 \{#defined-in\}

[react/client.ts:317](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L317)

## 访问器 \{#accessors\}

### url \{#url\}

• `get` **url**(): `string`

返回此客户端的地址，可用于创建新客户端。

不保证与构造该客户端时使用的地址完全一致：
该地址可能会被规范化处理。

#### 返回 \{#returns\}

`string`

#### 定义于 \{#defined-in\}

[react/client.ts:352](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L352)

***

### logger \{#logger\}

• `get` **logger**(): `Logger`

获取此客户端的日志记录器。

#### 返回值 \{#returns\}

`Logger`

该客户端的 Logger。

#### 定义于 \{#defined-in\}

[react/client.ts:713](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L713)

## 方法 \{#methods\}

### setAuth \{#setauth\}

▸ **setAuth**(`fetchToken`, `onChange?`): `void`

设置后续查询和变更要使用的身份验证令牌。
如果令牌过期，将会自动再次调用 `fetchToken`。
如果无法获取令牌（例如用户权限被永久撤销），`fetchToken` 应返回 `null`。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `fetchToken` | [`AuthTokenFetcher`](../modules/browser.md#authtokenfetcher) | 一个返回 JWT 编码的 OpenID Connect 身份令牌的异步函数 |
| `onChange?` | (`isAuthenticated`: `boolean`) =&gt; `void` | 当身份验证状态变化时会被调用的回调函数 |

#### 返回 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[react/client.ts:408](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L408)

***

### clearAuth \{#clearauth\}

▸ **clearAuth**(): `void`

如果已设置，则清除当前的身份验证令牌。

#### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[react/client.ts:430](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L430)

***

### watchQuery \{#watchquery\}

▸ **watchQuery**&lt;`Query`&gt;(`query`, `...argsAndOptions`): [`Watch`](../interfaces/react.Watch.md)&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

在 Convex 查询函数上构建一个新的 [Watch](../interfaces/react.Watch.md)。

**大多数应用代码不应直接调用此方法，而是应使用
[useQuery](../modules/react.md#usequery) hook。**

仅仅创建一个 watch 本身不会产生任何效果，Watch 是无状态的。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `query` | `Query` | 要执行的公共查询的 [FunctionReference](../modules/server.md#functionreference)。 |
| `...argsAndOptions` | [`ArgsAndOptions`](../modules/server.md#argsandoptions)&lt;`Query`, [`WatchQueryOptions`](../interfaces/react.WatchQueryOptions.md)&gt; | - |

#### 返回值 \{#returns\}

[`Watch`](../interfaces/react.Watch.md)&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

[Watch](../interfaces/react.Watch.md) 对象。

#### 定义于 \{#defined-in\}

[react/client.ts:463](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L463)

***

### prewarmQuery \{#prewarmquery\}

▸ **prewarmQuery**&lt;`Query`&gt;(`queryOptions`): `void`

表示预期后续很可能需要订阅某个查询。

当前的实现会立即订阅该查询。将来，这个方法可能会对某些查询进行优先处理、在不订阅的情况下预先获取查询结果，或者在网络连接较慢或高负载场景下什么都不做。

在 React 组件中使用时，可以调用 useQuery() 并忽略其返回值。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `queryOptions` | `ConvexQueryOptions`&lt;`Query`&gt; &amp; &#123; `extendSubscriptionFor?`: `number`  &#125; | 一个查询（来自 API 对象的函数引用）及其参数，再加上可选的 extendSubscriptionFor 字段，用于指定订阅该查询的时长。 |

#### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[react/client.ts:539](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L539)

***

### 变更 \{#mutation\}

▸ **mutation**&lt;`Mutation`&gt;(`mutation`, `...argsAndOptions`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

执行变更函数。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | 要运行的公开变更函数的 [FunctionReference](../modules/server.md#functionreference)。 |
| `...argsAndOptions` | [`ArgsAndOptions`](../modules/server.md#argsandoptions)&lt;`Mutation`, [`MutationOptions`](../interfaces/react.MutationOptions.md)&lt;[`FunctionArgs`](../modules/server.md#functionargs)&lt;`Mutation`&gt;&gt;&gt; | - |

#### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

一个最终解析为该变更结果的 Promise。

#### 定义于 \{#defined-in\}

[react/client.ts:618](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L618)

***

### 操作 \{#action\}

▸ **action**&lt;`Action`&gt;(`action`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

执行一个操作函数。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Action` | 继承自 [`FunctionReference`](../modules/server.md#functionreference)&lt;`"action"`&gt; |

#### 参数 \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `action` | `Action` | 要运行的公共操作的 [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Action`&gt; | 传递给该操作的参数对象。如果省略此参数，将使用 `{}`。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

一个解析为该操作结果的 Promise。

#### 定义于 \{#defined-in\}

[react/client.ts:639](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L639)

***

### query \{#query\}

▸ **query**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

仅获取一次查询结果。

**大多数应用代码应当使用 [useQuery](../modules/react.md#usequery) hook 来订阅查询，
而不是只获取一次结果。**

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `query` | `Query` | 要运行的公开查询对应的 [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; | 查询的参数对象。如果省略，则该参数为 `{}`。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

一个会解析为该查询结果的 Promise。

#### 定义于 \{#defined-in\}

[react/client.ts:659](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L659)

***

### connectionState \{#connectionstate\}

▸ **connectionState**(): [`ConnectionState`](../modules/browser.md#connectionstate)

获取客户端与 Convex 后端当前的 [ConnectionState](../modules/browser.md#connectionstate)。

#### 返回值 \{#returns\}

[`ConnectionState`](../modules/browser.md#connectionstate)

表示与 Convex 后端连接状态的 [ConnectionState](../modules/browser.md#connectionstate)。

#### 定义于 \{#defined-in\}

[react/client.ts:686](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L686)

***

### subscribeToConnectionState \{#subscribetoconnectionstate\}

▸ **subscribeToConnectionState**(`cb`): () =&gt; `void`

订阅客户端与 Convex 后端之间的 [ConnectionState](../modules/browser.md#connectionstate)，并在其每次发生变化时调用回调函数。

当 ConnectionState 的任何部分发生变化时，已订阅的回调函数都会被调用。
ConnectionState 在未来的版本中可能会扩展（例如提供一个包含进行中请求的数组），这种情况下回调会被更频繁地调用。
随着我们逐步了解哪些信息最有用，ConnectionState 在未来的版本中也可能*减少*某些属性。因此，此 API 被视为不稳定的。

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `cb` | (`connectionState`: [`ConnectionState`](../modules/browser.md#connectionstate)) =&gt; `void` |

#### 返回值 \{#returns\}

`fn`

用于停止监听的取消订阅函数。

▸ (): `void`

订阅客户端与 Convex 后端之间的 [ConnectionState](../modules/browser.md#connectionstate)，并在其每次变化时调用回调函数。

当 ConnectionState 的任何部分发生变化时，已订阅的回调都会被调用。
ConnectionState 在未来版本中可能会扩展（例如提供一个正在进行中的请求数组），在这种情况下回调会被更频繁地调用。
随着我们逐步明确哪些信息最有用，ConnectionState 在未来版本中也可能会*移除*某些属性。因此，此 API 被视为不稳定的。

##### 返回值 \{#returns\}

`void`

用于停止监听的取消订阅函数。

#### 定义于 \{#defined-in\}

[react/client.ts:702](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L702)

***

### close \{#close\}

▸ **close**(): `Promise`&lt;`void`&gt;

关闭与此客户端关联的所有网络连接，并停止所有订阅。

当你不再需要使用 [ConvexReactClient](react.ConvexReactClient.md) 时，调用此方法来释放其套接字和资源。

#### 返回值 \{#returns\}

`Promise`&lt;`void`&gt;

在连接完全关闭时被解决的 `Promise`。

#### 定义于 \{#defined-in\}

[react/client.ts:725](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L725)