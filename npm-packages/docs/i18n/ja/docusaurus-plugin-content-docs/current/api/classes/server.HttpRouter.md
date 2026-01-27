---
id: "server.HttpRouter"
title: "クラス: HttpRouter"
custom_edit_url: null
---

[server](../modules/server.md).HttpRouter

[httpActionGeneric](../modules/server.md#httpactiongeneric) のパスやメソッドを指定するための HTTP ルーターです。

たとえば、`convex/http.js` ファイルは次のようになります。

```js
import { httpRouter } from "convex/server";
import { getMessagesByAuthor } from "./getMessagesByAuthor";
import { httpAction } from "./_generated/server";

const http = httpRouter();

// HTTP actions can be defined inline...
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

// ...or they can be imported from other files.
http.route({
  path: "/getMessagesByAuthor",
  method: "GET",
  handler: getMessagesByAuthor,
});

// Convexは`convex/http.js`のデフォルトエクスポートとしてルーターを期待します。
export default http;
```

## コンストラクタ \{#constructors\}

### コンストラクタ \{#constructor\}

• **new HttpRouter**()

## プロパティ \{#properties\}

### exactRoutes \{#exactroutes\}

• **exactRoutes**: `Map`&lt;`string`, `Map`&lt;`"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, [`PublicHttpAction`](../modules/server.md#publichttpaction)&gt;&gt;

#### 定義元 \{#defined-in\}

[server/router.ts:143](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L143)

***

### prefixRoutes \{#prefixroutes\}

• **prefixRoutes**: `Map`&lt;`"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, `Map`&lt;`string`, [`PublicHttpAction`](../modules/server.md#publichttpaction)&gt;&gt;

#### 定義場所 \{#defined-in\}

[server/router.ts:144](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L144)

***

### isRouter \{#isrouter\}

• **isRouter**: `true`

#### 定義場所 \{#defined-in\}

[server/router.ts:145](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L145)

## メソッド \{#methods\}

### route \{#route\}

▸ **route**(`spec`): `void`

HTTP メソッド（例: &quot;GET&quot;）とパスまたは pathPrefix の組み合わせに対するリクエストに応答するために使用する HttpAction を指定します。

パスはスラッシュで始まっている必要があります。パスプレフィックスもスラッシュで終わっている必要があります。

```js
// matches `/profile` (but not `/profile/`)
http.route({ path: "/profile", method: "GET", handler: getProfile})

// `/profiles/`、`/profiles/abc`、`/profiles/a/c/b` にマッチします（`/profile` にはマッチしません）
http.route({ pathPrefix: "/profile/", method: "GET", handler: getProfile})
```

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `spec` | [`RouteSpec`](../modules/server.md#routespec) |

#### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[server/router.ts:161](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L161)

***

### getRoutes \{#getroutes\}

▸ **getRoutes**(): readonly [`string`, `"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, [`PublicHttpAction`](../modules/server.md#publichttpaction)][]

ルーティングされた HTTP アクションの一覧を返します。

これらは Convex ダッシュボードの Functions ページに表示されるルート一覧を生成するために使用されます。

#### 戻り値 \{#returns\}

readonly [`string`, `"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, [`PublicHttpAction`](../modules/server.md#publichttpaction)][]

* [path, method, endpoint] というタプルの配列。

#### 定義元 \{#defined-in\}

[server/router.ts:229](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L229)

***

### lookup \{#lookup\}

▸ **lookup**(`path`, `method`): `null` | readonly [[`PublicHttpAction`](../modules/server.md#publichttpaction), `"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, `string`]

対応する HTTP アクションと、そのアクションに対してルーティングされたリクエストパスおよびメソッドを返します。

返されるパスとメソッドはログおよびメトリクスに使用され、
`getRoutes` が返すルートのいずれかと一致している必要があります。

たとえば、

```js
http.route({ pathPrefix: "/profile/", method: "GET", handler: getProfile});

http.lookup("/profile/abc", "GET") // [getProfile, "GET", "/profile/*"] を返します
```

#### パラメータ \{#parameters\}

| パラメーター名 | 型 |
| :------ | :------ |
| `path` | `string` |
| `method` | `"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"` | `"HEAD"` |

#### Returns \{#returns\}

`null` | readonly [[`PublicHttpAction`](../modules/server.md#publichttpaction), `"GET"` | `"POST"` | `"PUT"` | `"DELETE"` | `"OPTIONS"` | `"PATCH"`, `string`]

* [[PublicHttpAction](../modules/server.md#publichttpaction), method, path] から成るタプル、または null。

#### 定義元 \{#defined-in\}

[server/router.ts:275](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L275)

***

### runRequest \{#runrequest\}

▸ **runRequest**(`argsStr`, `requestRoute`): `Promise`&lt;`string`&gt;

JSON 文字列で表現された Request オブジェクトを受け取り、そのリクエストをルーティングして適切なエンドポイントを実行するか、404 Response を返し、その結果の Response を返します。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `argsStr` | `string` | Request オブジェクトを表す JSON 文字列です。 |
| `requestRoute` | `string` | - |

#### 戻り値 \{#returns\}

`Promise`&lt;`string`&gt;

* Response オブジェクトを返します。

#### 定義場所 \{#defined-in\}

[server/router.ts:304](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L304)