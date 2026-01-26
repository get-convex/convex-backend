---
id: "react.ConvexReactClientOptions"
title: "接口：ConvexReactClientOptions"
custom_edit_url: null
---

[react](../modules/react.md).ConvexReactClientOptions

用于 [ConvexReactClient](../classes/react.ConvexReactClient.md) 的选项。

## 继承关系 \{#hierarchy\}

* [`BaseConvexClientOptions`](browser.BaseConvexClientOptions.md)

  ↳ **`ConvexReactClientOptions`**

## 属性 \{#properties\}

### unsavedChangesWarning \{#unsavedchangeswarning\}

• `Optional` **unsavedChangesWarning**: `boolean`

当用户有未保存的更改时，是否在其离开当前页面或关闭网页时进行提示。

这仅在 `window` 对象存在时才生效，即在浏览器中。

在浏览器中的默认值为 `true`。

#### 继承自 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[unsavedChangesWarning](browser.BaseConvexClientOptions.md#unsavedchangeswarning)

#### 定义在 \{#defined-in\}

[browser/sync/client.ts:69](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L69)

***

### webSocketConstructor \{#websocketconstructor\}

• `可选` **webSocketConstructor**: `Object`

#### 调用签名 \{#call-signature\}

• **new webSocketConstructor**(`url`, `protocols?`): `WebSocket`

指定一个备用的
[WebSocket](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)
构造函数，用于客户端与 Convex 云之间的通信。
默认情况下会使用全局环境中的 `WebSocket` 构造函数。

##### 参数 \{#parameters\}

| 名称 | 类型 |
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

#### 继承自 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[webSocketConstructor](browser.BaseConvexClientOptions.md#websocketconstructor)

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:76](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L76)

***

### verbose \{#verbose\}

• `Optional` **verbose**: `boolean`

启用额外日志输出以便调试。

默认值为 `false`。

#### 继承自 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[verbose](browser.BaseConvexClientOptions.md#verbose)

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:82](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L82)

***

### logger \{#logger\}

• `Optional` **logger**: `boolean` | `Logger`

可以是一个 logger、`true` 或 `false`。如果未提供或为 `true`，日志会输出到控制台。
如果为 `false`，则不会在任何地方打印日志。

你可以构造自己的 logger，将日志输出自定义到其他位置。
logger 是一个包含 4 个方法的对象：log()、warn()、error() 和 logVerbose()。
这些方法可以像 console.log() 一样接收任意类型的多个参数。

#### 继承自 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[logger](browser.BaseConvexClientOptions.md#logger)

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:91](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L91)

***

### reportDebugInfoToConvex \{#reportdebuginfotoconvex\}

• `Optional` **reportDebugInfoToConvex**: `boolean`

向 Convex 发送额外的指标数据用于调试。

默认值为 `false`。

#### 继承自 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[reportDebugInfoToConvex](browser.BaseConvexClientOptions.md#reportdebuginfotoconvex)

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:97](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L97)

***

### onServerDisconnectError \{#onserverdisconnecterror\}

• `Optional` **onServerDisconnectError**: (`message`: `string`) =&gt; `void`

#### 类型声明 \{#type-declaration\}

▸ (`message`): `void`

此 API 为实验性功能：其行为可能会变更或被移除。

这是一个在收到来自已连接 Convex 部署的异常 WebSocket 关闭消息时调用的函数。
这些消息的内容并不稳定，属于实现细节，未来可能会改变。

在更高层级的状态码与处理建议可用之前，可将此 API 视为用于可观测性的临时权宜之计；
这些更高层级状态码可能提供比 `string` 更稳定的接口。

查看 `connectionState` 以获取关于连接状态的更多量化指标。

##### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `message` | `string` |

##### 返回值 \{#returns\}

`void`

#### 继承自 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[onServerDisconnectError](browser.BaseConvexClientOptions.md#onserverdisconnecterror)

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:111](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L111)

***

### skipConvexDeploymentUrlCheck \{#skipconvexdeploymenturlcheck\}

• `Optional` **skipConvexDeploymentUrlCheck**: `boolean`

跳过验证 Convex 部署 URL 是否形如
`https://happy-animal-123.convex.cloud` 或 localhost。

当你运行使用不同 URL 的自托管 Convex 后端时，这会很有用。

默认值为 `false`

#### 继承自 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[skipConvexDeploymentUrlCheck](browser.BaseConvexClientOptions.md#skipconvexdeploymenturlcheck)

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:121](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L121)

***

### authRefreshTokenLeewaySeconds \{#authrefreshtokenleewayseconds\}

• `Optional` **authRefreshTokenLeewaySeconds**: `number`

如果使用 auth，此配置表示在令牌过期前多少秒开始刷新令牌。

默认值为 `2`。

#### 继承自 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[authRefreshTokenLeewaySeconds](browser.BaseConvexClientOptions.md#authrefreshtokenleewayseconds)

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:127](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L127)

***

### expectAuth \{#expectauth\}

• `Optional` **expectAuth**: `boolean`

此 API 为实验性质：其行为可能会变更或被移除。

是否在能够发送首个认证令牌之前，
延后发送查询、变更和操作请求。

为仅应由已认证客户端访问的页面启用此行为效果很好。

默认值为 false，即不会等待认证令牌。

#### 继承自 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[expectAuth](browser.BaseConvexClientOptions.md#expectauth)

#### 定义于 \{#defined-in\}

[browser/sync/client.ts:139](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L139)