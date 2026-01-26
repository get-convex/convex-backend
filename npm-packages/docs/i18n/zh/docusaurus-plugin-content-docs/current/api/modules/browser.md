---
id: "browser"
title: "模块：browser"
custom_edit_url: null
---

在浏览器环境中访问 Convex 的工具。

**如果你在使用 React，请改用 [react](react.md) 模块。**

## 用法 \{#usage\}

创建一个 [ConvexHttpClient](../classes/browser.ConvexHttpClient.md) 来连接 Convex Cloud。

```typescript
import { ConvexHttpClient } from "convex/browser";
// 通常从环境变量中加载
const address = "https://small-mouse-123.convex.cloud";
const convex = new ConvexHttpClient(address);
```

## 类 \{#classes\}

* [ConvexHttpClient](../classes/browser.ConvexHttpClient.md)
* [ConvexClient](../classes/browser.ConvexClient.md)
* [BaseConvexClient](../classes/browser.BaseConvexClient.md)

## 接口 \{#interfaces\}

* [BaseConvexClientOptions](../interfaces/browser.BaseConvexClientOptions.md)
* [SubscribeOptions](../interfaces/browser.SubscribeOptions.md)
* [MutationOptions](../interfaces/browser.MutationOptions.md)
* [OptimisticLocalStore](../interfaces/browser.OptimisticLocalStore.md)

## 类型别名 \{#type-aliases\}

### HttpMutationOptions \{#httpmutationoptions\}

Ƭ **HttpMutationOptions**: `Object`

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `skipQueue` | `boolean` | 跳过默认的变更队列并立即执行该变更。这使你可以使用同一个 HttpConvexClient 并行请求多个变更，而基于 WebSocket 的客户端无法做到这一点。 |

#### 定义于 \{#defined-in\}

[browser/http&#95;client.ts:40](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L40)

***

### ConvexClientOptions \{#convexclientoptions\}

Ƭ **ConvexClientOptions**: [`BaseConvexClientOptions`](../interfaces/browser.BaseConvexClientOptions.md) &amp; &#123; `disabled?`: `boolean` ; `unsavedChangesWarning?`: `boolean`  &#125;

#### 定义于 \{#defined-in\}

[browser/simple&#95;client.ts:36](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L36)

***

### AuthTokenFetcher \{#authtokenfetcher\}

Ƭ **AuthTokenFetcher**: (`args`: &#123; `forceRefreshToken`: `boolean`  &#125;) =&gt; `Promise`&lt;`string` | `null` | `undefined`&gt;

#### 类型声明 \{#type-declaration\}

▸ (`args`): `Promise`&lt;`string` | `null` | `undefined`&gt;

一个异步函数，用于返回 JWT。根据在 convex/auth.config.ts 中配置的身份验证提供商，
它可能是一个通过 JWT 编码的 OpenID Connect 身份令牌，也可能是一个传统的 JWT。

当服务器拒绝了之前返回的令牌，或者根据其 `exp` 时间预计令牌即将过期时，
`forceRefreshToken` 为 `true`。

参见 ConvexReactClient.setAuth。

##### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `args` | `Object` |
| `args.forceRefreshToken` | `boolean` |

##### 返回值 \{#returns\}

`Promise`&lt;`string` | `null` | `undefined`&gt;

#### 定义于 \{#defined-in\}

[browser/sync/authentication&#95;manager.ts:25](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/authentication_manager.ts#L25)

***

### ConnectionState \{#connectionstate\}

Ƭ **ConnectionState**: `Object`

表示客户端与 Convex 后端连接状态的对象。

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `hasInflightRequests` | `boolean` | - |
| `isWebSocketConnected` | `boolean` | - |
| `timeOfOldestInflightRequest` | `Date` | `null` | - |
| `hasEverConnected` | `boolean` | 如果客户端曾经成功将 WebSocket 连接到 &quot;ready&quot; 状态，则为 true。 |
| `connectionCount` | `number` | 此客户端连接到 Convex 后端的次数。多种情况都可能导致客户端重新连接 —— 服务器错误、网络状况不佳、认证过期。但如果这个数字很高，则表明客户端在维持稳定连接方面遇到问题。 |
| `connectionRetries` | `number` | 此客户端尝试（并失败）连接到 Convex 后端的次数。 |
| `inflightMutations` | `number` | 当前正在进行中的变更数量。 |
| `inflightActions` | `number` | 当前正在进行中的操作函数数量。 |

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:147](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L147)

***

### FunctionResult \{#functionresult\}

Ƭ **FunctionResult**: `FunctionSuccess` | `FunctionFailure`

在服务器上运行函数的结果。

如果函数抛出了异常，它会包含一个 `errorMessage`。否则，它会返回一个 `Value`。

#### 定义于 \{#defined-in\}

[browser/sync/function&#95;result.ts:11](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/function_result.ts#L11)

***

### OptimisticUpdate \{#optimisticupdate\}

Ƭ **OptimisticUpdate**&lt;`Args`&gt;: (`localQueryStore`: [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md), `args`: `Args`) =&gt; `void`

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Args` | extends `Record`&lt;`string`, [`Value`](values.md#value)&gt; |

#### 类型声明 \{#type-declaration\}

▸ (`localQueryStore`, `args`): `void`

对该客户端内的查询结果进行一次临时的本地更新。

当某个变更与 Convex 服务器同步时，此更新都会被执行，并在该变更完成后回滚。

注意，乐观更新可以被调用多次！如果客户端在变更进行期间加载了新的数据，该更新会被再次重放。

##### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `localQueryStore` | [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md) | 用于读取和编辑本地查询结果的接口。 |
| `args` | `Args` | 变更的参数。 |

##### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[browser/sync/optimistic&#95;updates.ts:90](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/optimistic_updates.ts#L90)

***

### PaginationStatus \{#paginationstatus\}

Ƭ **PaginationStatus**: `"LoadingFirstPage"` | `"CanLoadMore"` | `"LoadingMore"` | `"Exhausted"`

#### 定义于 \{#defined-in\}

[browser/sync/pagination.ts:5](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/pagination.ts#L5)

***

### QueryJournal \{#queryjournal\}

Ƭ **QueryJournal**: `string` | `null`

对查询执行期间所做决策的序列化表示。

在查询函数首次执行时会生成一个日志（journal），在查询被重新执行时会复用该日志。

目前它用于存储分页的结束游标，以确保分页查询的各个页面始终在同一游标结束，
从而实现无间隙的响应式分页。

`null` 用于表示空日志。

#### 定义于 \{#defined-in\}

[browser/sync/protocol.ts:113](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/protocol.ts#L113)

***

### QueryToken \{#querytoken\}

Ƭ **QueryToken**: `string` &amp; &#123; `__queryToken`: `true`  &#125;

表示查询名称和参数的字符串。

该类型由 [BaseConvexClient](../classes/browser.BaseConvexClient.md) 使用。

#### 定义于 \{#defined-in\}

[browser/sync/udf&#95;path&#95;utils.ts:31](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/udf_path_utils.ts#L31)

***

### PaginatedQueryToken \{#paginatedquerytoken\}

Ƭ **PaginatedQueryToken**: [`QueryToken`](browser.md#querytoken) &amp; &#123; `__paginatedQueryToken`: `true`  &#125;

表示分页查询名称和参数的字符串。

这是用于分页查询的 QueryToken 的一种专用形式。

#### 定义于 \{#defined-in\}

[browser/sync/udf&#95;path&#95;utils.ts:38](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/udf_path_utils.ts#L38)

***

### UserIdentityAttributes \{#useridentityattributes\}

Ƭ **UserIdentityAttributes**: `Omit`&lt;[`UserIdentity`](../interfaces/server.UserIdentity.md), `"tokenIdentifier"`&gt;

#### 定义于 \{#defined-in\}

[server/authentication.ts:215](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L215)