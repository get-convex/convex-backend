---
id: "browser.ConvexClient"
title: "类：ConvexClient"
custom_edit_url: null
---

[browser](../modules/browser.md).ConvexClient

通过 WebSocket 订阅 Convex 查询函数，并执行变更函数和操作函数。

此客户端不提供对变更的乐观更新。
第三方客户端可以选择封装 [BaseConvexClient](browser.BaseConvexClient.md) 以获得额外的控制能力。

```ts
const client = new ConvexClient("https://happy-otter-123.convex.cloud");
const unsubscribe = client.onUpdate(api.messages.list, {}, (messages) => {
  console.log(messages[0].body);
});
```

## 构造函数 \{#constructors\}

### constructor \{#constructor\}

• **new ConvexClient**(`address`, `options?`)

构造一个客户端实例，并立即建立到传入地址的 WebSocket 连接。

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `address` | `string` |
| `options` | [`ConvexClientOptions`](../modules/browser.md#convexclientoptions) |

#### 定义于 \{#defined-in\}

[browser/simple&#95;client.ts:119](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L119)

## 访问器 \{#accessors\}

### closed \{#closed\}

• `get` **closed**(): `boolean`

一旦关闭，任何已注册的回调都不会再被触发。

#### 返回值 \{#returns\}

`boolean`

#### 定义于 \{#defined-in\}

[browser/simple&#95;client.ts:96](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L96)

***

### client \{#client\}

• `get` **client**(): [`BaseConvexClient`](browser.BaseConvexClient.md)

#### 返回值 \{#returns\}

[`BaseConvexClient`](browser.BaseConvexClient.md)

#### 定义于 \{#defined-in\}

[browser/simple&#95;client.ts:99](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L99)

***

### disabled \{#disabled\}

• `get` **disabled**(): `boolean`

#### 返回 \{#returns\}

`boolean`

#### 定义于 \{#defined-in\}

[browser/simple&#95;client.ts:110](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L110)

## 方法 \{#methods\}

### onUpdate \{#onupdate\}

▸ **onUpdate**&lt;`Query`&gt;(`query`, `args`, `callback`, `onError?`): `Unsubscribe`&lt;`Query`[`"_returnType"`]&gt;

每当收到某个查询的新结果时，就会调用回调函数。如果该查询的结果已经在内存中，回调会在注册后不久自动执行。

返回值是一个 `Unsubscribe` 对象，它既是一个函数，又是一个带有属性的对象。下面两种用法对这个对象都适用：

```ts
// call the return value as a function
const unsubscribe = client.onUpdate(api.messages.list, {}, (messages) => {
  console.log(messages);
});
unsubscribe();

// 将返回值解构为其属性
const {
  getCurrentValue,
  unsubscribe,
} = client.onUpdate(api.messages.list, {}, (messages) => {
  console.log(messages);
});
```

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `query` | `Query` | 要运行的公共查询的 [FunctionReference](../modules/server.md#functionreference)。 |
| `args` | [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Query`&gt; | 运行该查询时使用的参数。 |
| `callback` | (`result`: [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;) =&gt; `unknown` | 查询结果更新时调用的函数。 |
| `onError?` | (`e`: `Error`) =&gt; `unknown` | 当查询结果以错误形式更新时调用的函数。如果未提供，将抛出错误，而不是调用回调函数。 |

#### 返回值 \{#returns\}

`Unsubscribe`&lt;`Query`[`"_returnType"`]&gt;

一个用于停止调用 onUpdate 函数的 Unsubscribe 函数。

#### 定义于 \{#defined-in\}

[browser/simple&#95;client.ts:185](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L185)

***

### onPaginatedUpdate_experimental \{#onpaginatedupdate_experimental\}

▸ **onPaginatedUpdate&#95;experimental**&lt;`Query`&gt;(`query`, `args`, `options`, `callback`, `onError?`): `Unsubscribe`&lt;`PaginatedQueryResult`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;[]&gt;&gt;

每当收到某个分页查询的新结果时，调用一次回调函数。

这是一个实验性预览功能：最终的 API 可能会发生变化。
尤其是缓存行为、分页拆分方式以及分页查询所需的选项
都可能会变更。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `query` | `Query` | 表示要运行的公共查询的 [FunctionReference](../modules/server.md#functionreference)。 |
| `args` | [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Query`&gt; | 用于运行该查询的参数。 |
| `options` | `Object` | 分页查询的配置选项，包括 initialNumItems 和 id。 |
| `options.initialNumItems` | `number` | - |
| `callback` | (`result`: [`PaginationResult`](../interfaces/server.PaginationResult.md)&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;) =&gt; `unknown` | 在查询结果更新时调用的函数。 |
| `onError?` | (`e`: `Error`) =&gt; `unknown` | 在查询结果以错误形式更新时调用的函数。 |

#### Returns \{#returns\}

`Unsubscribe`&lt;`PaginatedQueryResult`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;[]&gt;&gt;

一个用于停止调用该回调的 Unsubscribe 函数。

#### 定义于 \{#defined-in\}

[browser/simple&#95;client.ts:263](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L263)

***

### close \{#close\}

▸ **close**(): `Promise`&lt;`void`&gt;

#### 返回值 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定义于 \{#defined-in\}

[browser/simple&#95;client.ts:366](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L366)

***

### getAuth \{#getauth\}

▸ **getAuth**(): `undefined` | &#123; `token`: `string` ; `decoded`: `Record`&lt;`string`, `any`&gt;  &#125;

获取当前的 JWT 身份验证令牌及其解码后的声明。

#### 返回值 \{#returns\}

`undefined` | &#123; `token`: `string` ; `decoded`: `Record`&lt;`string`, `any`&gt;  &#125;

#### 定义于 \{#defined-in\}

[browser/simple&#95;client.ts:380](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L380)

***

### setAuth \{#setauth\}

▸ **setAuth**(`fetchToken`, `onChange?`): `void`

设置在后续查询和变更函数中使用的身份验证令牌。
如果令牌过期，`fetchToken` 会被自动再次调用。
当无法获取令牌时，`fetchToken` 应返回 `null`，例如用户权限已被永久撤销时。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `fetchToken` | [`AuthTokenFetcher`](../modules/browser.md#authtokenfetcher) | 一个返回 JWT 的异步函数（通常是 OpenID Connect 身份令牌） |
| `onChange?` | (`isAuthenticated`: `boolean`) =&gt; `void` | 当身份验证状态发生变化时会被调用的回调函数 |

#### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[browser/simple&#95;client.ts:393](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L393)

***

### mutation \{#mutation\}

▸ **mutation**&lt;`Mutation`&gt;(`mutation`, `args`, `options?`): `Promise`&lt;`Awaited`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;&gt;

执行变更函数。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | 要运行的公共变更的 [FunctionReference](../modules/server.md#functionreference)。 |
| `args` | [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Mutation`&gt; | 该变更的参数对象。 |
| `options?` | [`MutationOptions`](../interfaces/browser.MutationOptions.md) | 用于该变更的 [MutationOptions](../interfaces/browser.MutationOptions.md) 配置对象。 |

#### 返回值 \{#returns\}

`Promise`&lt;`Awaited`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;&gt;

表示该变更结果的 `Promise`。

#### 定义于 \{#defined-in\}

[browser/simple&#95;client.ts:488](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L488)

***

### action \{#action\}

▸ **action**&lt;`Action`&gt;(`action`, `args`): `Promise`&lt;`Awaited`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;&gt;

执行操作函数。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"action"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `action` | `Action` | 要执行的公开操作的 [FunctionReference](../modules/server.md#functionreference)。 |
| `args` | [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Action`&gt; | 操作的参数对象。 |

#### 返回值 \{#returns\}

`Promise`&lt;`Awaited`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;&gt;

表示该操作结果的 Promise。

#### 定义于 \{#defined-in\}

[browser/simple&#95;client.ts:505](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L505)

***

### query \{#query\}

▸ **query**&lt;`Query`&gt;(`query`, `args`): `Promise`&lt;`Awaited`&lt;`Query`[`"_returnType"`]&gt;&gt;

获取单次查询结果。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `query` | `Query` | 要运行的公共查询的 [FunctionReference](../modules/server.md#functionreference)。 |
| `args` | `Query`[`"_args"`] | 查询的参数对象。 |

#### 返回值 \{#returns\}

`Promise`&lt;`Awaited`&lt;`Query`[`"_returnType"`]&gt;&gt;

一个表示查询结果的 Promise。

#### 定义于 \{#defined-in\}

[browser/simple&#95;client.ts:521](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L521)

***

### connectionState \{#connectionstate\}

▸ **connectionState**(): [`ConnectionState`](../modules/browser.md#connectionstate)

获取客户端与 Convex 后端之间当前的 [ConnectionState](../modules/browser.md#connectionstate)。

#### 返回值 \{#returns\}

[`ConnectionState`](../modules/browser.md#connectionstate)

表示与 Convex 后端连接状态的 [ConnectionState](../modules/browser.md#connectionstate)。

#### 定义于 \{#defined-in\}

[browser/simple&#95;client.ts:553](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L553)

***

### subscribeToConnectionState \{#subscribetoconnectionstate\}

▸ **subscribeToConnectionState**(`cb`): () =&gt; `void`

订阅客户端与 Convex 后端之间的 [ConnectionState](../modules/browser.md#connectionstate)，并在其每次变化时调用回调函数。

当 ConnectionState 的任意部分发生变化时，已订阅的回调函数都会被调用。
ConnectionState 在未来的版本中可能会扩展（例如提供一个包含正在进行请求的数组），届时回调函数将会更频繁地被调用。

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `cb` | (`connectionState`: [`ConnectionState`](../modules/browser.md#connectionstate)) =&gt; `void` |

#### 返回值 \{#returns\}

`fn`

用于停止监听的取消订阅函数。

▸ (): `void`

订阅客户端与 Convex 后端之间的 [ConnectionState](../modules/browser.md#connectionstate)，
并在其每次变化时调用回调函数。

当 ConnectionState 的任意部分发生变化时，已订阅的回调都会被调用。
ConnectionState 在未来版本中可能会扩展（例如提供一个正在进行中的请求数组），
届时回调会更频繁地被调用。

##### 返回值 \{#returns\}

`void`

用于停止监听的取消订阅函数。

#### 定义于 \{#defined-in\}

[browser/simple&#95;client.ts:568](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L568)