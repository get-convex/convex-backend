---
id: "browser.BaseConvexClient"
title: "类：BaseConvexClient"
custom_edit_url: null
---

[browser](../modules/browser.md).BaseConvexClient

用于将状态管理库与 Convex 直接集成的底层客户端。

大多数开发者应使用更高级别的客户端，例如
[ConvexHttpClient](browser.ConvexHttpClient.md) 或基于 React Hook 的 [ConvexReactClient](react.ConvexReactClient.md)。

## 构造函数 \{#constructors\}

### 构造函数 \{#constructor\}

• **new BaseConvexClient**(`address`, `onTransition`, `options?`)

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `address` | `string` | Convex 部署的 URL，通常通过环境变量提供。例如：`https://small-mouse-123.convex.cloud`。 |
| `onTransition` | (`updatedQueries`: [`QueryToken`](../modules/browser.md#querytoken)[]) =&gt; `void` | 一个回调函数，接收一个查询 token 数组，这些 token 对应的查询结果已发生变化——可以通过 `addOnTransitionHandler` 添加额外的处理函数。 |
| `options?` | [`BaseConvexClientOptions`](../interfaces/browser.BaseConvexClientOptions.md) | 完整说明请参见 [BaseConvexClientOptions](../interfaces/browser.BaseConvexClientOptions.md)。 |

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:277](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L277)

## 访问器 \{#accessors\}

### url \{#url\}

• `get` **url**(): `string`

返回此客户端的地址，可用于创建新的客户端。

不能保证与构造此客户端时使用的地址完全一致：
该地址可能已被标准化处理。

#### 返回值 \{#returns\}

`string`

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:1037](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L1037)

## 方法 \{#methods\}

### getMaxObservedTimestamp \{#getmaxobservedtimestamp\}

▸ **getMaxObservedTimestamp**(): `undefined` | `Long`

#### 返回值 \{#returns\}

`undefined` | `Long`

#### 定义在 \{#defined-in\}

[browser/sync/client.ts:542](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L542)

***

### addOnTransitionHandler \{#addontransitionhandler\}

▸ **addOnTransitionHandler**(`fn`): () =&gt; `boolean`

添加一个在过渡发生时会被调用的处理函数。

任何外部副作用（例如设置 React state）都应在这里处理。

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `fn` | (`transition`: `Transition`) =&gt; `void` |

#### 返回值 \{#returns\}

`fn`

▸ (): `boolean`

##### 返回值 \{#returns\}

`boolean`

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:621](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L621)

***

### getCurrentAuthClaims \{#getcurrentauthclaims\}

▸ **getCurrentAuthClaims**(): `undefined` | &#123; `token`: `string` ; `decoded`: `Record`&lt;`string`, `any`&gt;  &#125;

获取当前 JWT 认证令牌以及解码后的声明。

#### 返回值 \{#returns\}

`undefined` | &#123; `token`: `string` ; `decoded`: `Record`&lt;`string`, `any`&gt;  &#125;

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:630](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L630)

***

### setAuth \{#setauth\}

▸ **setAuth**(`fetchToken`, `onChange`): `void`

设置用于后续查询和变更函数的认证令牌。
当令牌过期时，`fetchToken` 会被自动重新调用。
如果无法获取令牌，例如当用户的权限被永久撤销时，`fetchToken` 应该返回 `null`。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `fetchToken` | [`AuthTokenFetcher`](../modules/browser.md#authtokenfetcher) | 一个用于返回使用 JWT 编码的 OpenID Connect 身份令牌的异步函数 |
| `onChange` | (`isAuthenticated`: `boolean`) =&gt; `void` | 当认证状态发生变化时调用的回调函数 |

#### 返回值 \{#returns\}

`void`

#### 定义在 \{#defined-in\}

[browser/sync/client.ts:655](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L655)

***

### hasAuth \{#hasauth\}

▸ **hasAuth**(): `boolean`

#### 返回值 \{#returns\}

`boolean`

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:662](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L662)

***

### clearAuth \{#clearauth\}

▸ **clearAuth**(): `void`

#### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:672](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L672)

***

### subscribe \{#subscribe\}

▸ **subscribe**(`name`, `args?`, `options?`): `Object`

订阅一个查询函数。

每当此查询的结果发生变化时，传入构造函数的 `onTransition` 回调函数就会被调用。

#### 参数 \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `string` | 查询名称。 |
| `args?` | `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; | 查询的参数对象。如果省略，则默认为 `{}`。 |
| `options?` | [`SubscribeOptions`](../interfaces/browser.SubscribeOptions.md) | 此查询的 [SubscribeOptions](../interfaces/browser.SubscribeOptions.md) 选项对象。 |

#### 返回值 \{#returns\}

`Object`

一个对象，包含与此查询对应的 [QueryToken](../modules/browser.md#querytoken) 以及一个 `unsubscribe` 回调。

| 名称 | 类型 |
| :------ | :------ |
| `queryToken` | [`QueryToken`](../modules/browser.md#querytoken) |
| `unsubscribe` | () =&gt; `void` |

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:691](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L691)

***

### localQueryResult \{#localqueryresult\}

▸ **localQueryResult**(`udfPath`, `args?`): `undefined` | [`Value`](../modules/values.md#value)

仅基于当前本地状态的查询结果。

只有在我们已经订阅了该查询，或者该查询的值已通过乐观更新方式设置时，它才会返回值。

#### 参数 \{#parameters\}

| 参数名 | 类型 |
| :------ | :------ |
| `udfPath` | `string` |
| `args?` | `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; |

#### 返回值 \{#returns\}

`undefined` | [`值`](../modules/values.md#value)

#### 定义在 \{#defined-in\}

[browser/sync/client.ts:724](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L724)

***

### queryJournal \{#queryjournal\}

▸ **queryJournal**(`name`, `args?`): `undefined` | [`QueryJournal`](../modules/browser.md#queryjournal)

获取当前查询函数的 [QueryJournal](../modules/browser.md#queryjournal)。

如果我们尚未收到该查询的结果，则返回 `undefined`。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `name` | `string` | 查询名称。 |
| `args?` | `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; | 此查询的参数对象。 |

#### 返回值 \{#returns\}

`undefined` | [`QueryJournal`](../modules/browser.md#queryjournal)

此查询的 [QueryJournal](../modules/browser.md#queryjournal) 或 `undefined`。

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:777](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L777)

***

### connectionState \{#connectionstate\}

▸ **connectionState**(): [`ConnectionState`](../modules/browser.md#connectionstate)

获取当前客户端与 Convex 后端之间的连接状态 [`ConnectionState`](../modules/browser.md#connectionstate)。

#### 返回 \{#returns\}

[`ConnectionState`](../modules/browser.md#connectionstate)

与 Convex 后端之间的 [ConnectionState](../modules/browser.md#connectionstate)。

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:792](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L792)

***

### subscribeToConnectionState \{#subscribetoconnectionstate\}

▸ **subscribeToConnectionState**(`cb`): () =&gt; `void`

订阅客户端与 Convex 后端之间的 [ConnectionState](../modules/browser.md#connectionstate)，
并在其每次发生变化时调用回调函数。

当 ConnectionState 的任一部分发生变化时，已订阅的回调都会被调用。
ConnectionState 在未来版本中可能会扩展（例如，提供一个进行中请求的数组），
在这种情况下，回调会被更频繁地调用。

#### 参数 \{#parameters\}

| 参数名 | 类型 |
| :------ | :------ |
| `cb` | (`connectionState`: [`ConnectionState`](../modules/browser.md#connectionstate)) =&gt; `void` |

#### 返回值 \{#returns\}

`fn`

用于停止监听的取消订阅函数。

▸ (): `void`

订阅客户端与 Convex 后端之间的 [ConnectionState](../modules/browser.md#connectionstate)，
并在其每次发生变化时调用回调函数。

当 ConnectionState 的任何部分发生变化时，已订阅的回调都会被调用。
ConnectionState 在未来版本中可能会扩展（例如提供正在进行中的请求数组），
在这种情况下，回调会被更频繁地调用。

##### 返回值 \{#returns\}

`void`

用于取消订阅以停止监听的函数。

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:838](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L838)

***

### 变更 \{#mutation\}

▸ **mutation**(`name`, `args?`, `options?`): `Promise`&lt;`any`&gt;

执行一个变更函数。

#### 参数 \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `string` | 变更的名称。 |
| `args?` | `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; | 该变更的参数对象。如果省略，将默认为 `{}`。 |
| `options?` | [`MutationOptions`](../interfaces/browser.MutationOptions.md) | 此变更对应的 [MutationOptions](../interfaces/browser.MutationOptions.md) 配置对象。 |

#### 返回 \{#returns\}

`Promise`&lt;`any`&gt;

* 返回变更结果的 Promise。

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:858](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L858)

***

### action \{#action\}

▸ **action**(`name`, `args?`): `Promise`&lt;`any`&gt;

执行一个操作函数。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `name` | `string` | 操作的名称。 |
| `args?` | `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; | 操作的参数对象。如果省略，则该参数默认为 `{}`。 |

#### 返回值 \{#returns\}

`Promise`&lt;`any`&gt;

一个解析为该操作结果的 Promise。

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:979](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L979)

***

### close \{#close\}

▸ **close**(): `Promise`&lt;`void`&gt;

关闭与此客户端关联的所有网络连接，并停止所有订阅。

当你不再需要使用 [BaseConvexClient](browser.BaseConvexClient.md) 时调用此方法，
以释放其套接字和相关资源。

#### 返回值 \{#returns\}

`Promise`&lt;`void`&gt;

在连接完全关闭后会被 resolve 的 `Promise`。

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:1026](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L1026)