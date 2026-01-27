---
id: "server.Auth"
title: "接口：Auth"
custom_edit_url: null
---

[server](../modules/server.md).Auth

一个用于在 Convex 的查询和变更函数中访问当前已认证用户信息的接口。

## 方法 \{#methods\}

### getUserIdentity \{#getuseridentity\}

▸ **getUserIdentity**(): `Promise`&lt;`null` | [`UserIdentity`](server.UserIdentity.md)&gt;

获取当前已通过身份验证的用户的详细信息。

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`UserIdentity`](server.UserIdentity.md)&gt;

一个 `Promise`，如果 Convex 客户端配置了有效的 ID token，则会解析为 [UserIdentity](server.UserIdentity.md)，否则会：

* 在 Convex 查询、变更函数和操作函数中返回 `null`。
* 在 HTTP 操作函数中抛出异常（`throw`）。

#### 定义于 \{#defined-in\}

[server/authentication.ts:236](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L236)