---
id: "browser.BaseConvexClientOptions"
title: "接口：BaseConvexClientOptions"
custom_edit_url: null
---

[browser](../modules/browser.md).BaseConvexClientOptions

用于 [BaseConvexClient](../classes/browser.BaseConvexClient.md) 的配置选项。

## 层次结构 \{#hierarchy\}

* **`BaseConvexClientOptions`**

  ↳ [`ConvexReactClientOptions`](react.ConvexReactClientOptions.md)

## 属性 \{#properties\}

### unsavedChangesWarning \{#unsavedchangeswarning\}

• `Optional` **unsavedChangesWarning**: `boolean`

当用户存在未保存的更改时，
是否在其离开当前页面或关闭网页时提示。

这只在 `window` 对象存在时才会生效，也就是在浏览器环境中。

在浏览器中，默认值为 `true`。

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:69](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L69)

***

### webSocketConstructor \{#websocketconstructor\}

• `可选` **webSocketConstructor**: `Object`

#### 调用签名 \{#call-signature\}

• **new webSocketConstructor**(`url`, `protocols?`): `WebSocket`

指定一个用于客户端与 Convex 云进行通信的备用
[WebSocket](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)
构造函数。默认情况下使用全局环境提供的 `WebSocket`。

##### 参数 \{#parameters\}

| 参数名 | 类型 |
| :------ | :------ |
| `url` | `string` | `URL` |
| `protocols?` | `string` | `string`[] |

##### 返回值 \{#returns\}

`WebSocket`

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 |
| :------ | :------ |
| `prototype` | `WebSocket` |
| `CONNECTING` | `0` |
| `OPEN` | `1` |
| `CLOSING` | `2` |
| `CLOSED` | `3` |

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:76](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L76)

***

### verbose \{#verbose\}

• `Optional` **verbose**: `boolean`

启用额外日志输出，便于调试。

默认值为 `false`。

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:82](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L82)

***

### logger \{#logger\}

• `Optional` **logger**: `boolean` | `Logger`

一个 logger、`true` 或 `false`。如果未提供或为 `true`，则会将日志输出到控制台。
如果为 `false`，日志不会在任何地方打印。

你可以实现你自己的 logger，将日志输出到其他位置。
logger 是一个包含 4 个方法的对象：log()、warn()、error() 和 logVerbose()。
这些方法可以像 console.log() 一样接收任意类型的多个参数。

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:91](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L91)

***

### reportDebugInfoToConvex \{#reportdebuginfotoconvex\}

• `Optional` **reportDebugInfoToConvex**: `boolean`

向 Convex 发送额外的指标用于调试用途。

默认值为 `false`。

#### 定义在 \{#defined-in\}

[browser/sync/client.ts:97](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L97)

***

### onServerDisconnectError \{#onserverdisconnecterror\}

• `可选` **onServerDisconnectError**: (`message`: `string`) =&gt; `void`

#### 类型声明 \{#type-declaration\}

▸ (`message`): `void`

此 API 为实验性：其行为可能会更改或被移除。

一个在收到来自已连接 Convex 部署的异常 WebSocket 关闭消息时调用的函数。
这些消息的内容并不稳定，属于实现细节，未来可能改变。

在更高层级、带有处理建议的错误码可用之前，你可以将此 API 视为一种用于观测性的临时权宜方案，
未来这些错误码可能通过比 `string` 更稳定的接口提供。

查看 `connectionState` 以获取关于连接状态的更多量化指标。

##### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `message` | `string` |

##### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:111](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L111)

***

### skipConvexDeploymentUrlCheck \{#skipconvexdeploymenturlcheck\}

• `Optional` **skipConvexDeploymentUrlCheck**: `boolean`

跳过检查 Convex 部署 URL 的格式是否类似于
`https://happy-animal-123.convex.cloud` 或 localhost。

当你运行使用不同 URL 的自托管 Convex 后端时，这会很有用。

默认值为 `false`。

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:121](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L121)

***

### authRefreshTokenLeewaySeconds \{#authrefreshtokenleewayseconds\}

• `Optional` **authRefreshTokenLeewaySeconds**: `number`

如果使用 auth，表示在 token 过期前需要提前多少秒刷新它。

默认值为 `2`。

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:127](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L127)

***

### expectAuth \{#expectauth\}

• `Optional` **expectAuth**: `boolean`

此 API 为实验性特性：它可能会更改或被移除。

是否应在能发送第一个 auth token 之前
暂缓处理查询、变更和操作请求。

启用此行为非常适合那些
只应由已通过身份验证的客户端查看的页面。

默认为 false，即不会等待 auth token。

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:139](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L139)