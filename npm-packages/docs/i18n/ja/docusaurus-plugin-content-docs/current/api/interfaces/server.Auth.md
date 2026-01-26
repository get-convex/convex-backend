---
id: "server.Auth"
title: "インターフェイス: Auth"
custom_edit_url: null
---

[server](../modules/server.md).Auth

Convex のクエリ関数およびミューテーション関数内で、
現在認証されているユーザーに関する情報にアクセスするためのインターフェイスです。

## メソッド \{#methods\}

### getUserIdentity \{#getuseridentity\}

▸ **getUserIdentity**(): `Promise`&lt;`null` | [`UserIdentity`](server.UserIdentity.md)&gt;

現在認証済みのユーザーに関する情報を取得します。

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | [`UserIdentity`](server.UserIdentity.md)&gt;

Convex クライアントが有効な ID トークンで設定されている場合は、[UserIdentity](server.UserIdentity.md) で解決される `Promise`。そうでない場合は次のようになります:

* Convex のクエリ、ミューテーション、アクションでは `null` を返します。
* HTTP アクションでは `throw` します。

#### 定義場所 \{#defined-in\}

[server/authentication.ts:236](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L236)