---
id: "browser"
title: "モジュール: browser"
custom_edit_url: null
---

ブラウザで Convex にアクセスするためのツール群です。

**React を使用している場合は、代わりに [react](react.md) モジュールを使用してください。**

## 使用方法 \{#usage\}

Convex Cloud に接続するために [ConvexHttpClient](../classes/browser.ConvexHttpClient.md) を作成します。

```typescript
import { ConvexHttpClient } from "convex/browser";
// 通常は環境変数から読み込まれます
const address = "https://small-mouse-123.convex.cloud";
const convex = new ConvexHttpClient(address);
```

## クラス \{#classes\}

* [ConvexHttpClient](../classes/browser.ConvexHttpClient.md)
* [ConvexClient](../classes/browser.ConvexClient.md)
* [BaseConvexClient](../classes/browser.BaseConvexClient.md)

## インターフェース \{#interfaces\}

* [BaseConvexClientOptions](../interfaces/browser.BaseConvexClientOptions.md)
* [SubscribeOptions](../interfaces/browser.SubscribeOptions.md)
* [MutationOptions](../interfaces/browser.MutationOptions.md)
* [OptimisticLocalStore](../interfaces/browser.OptimisticLocalStore.md)

## 型エイリアス \{#type-aliases\}

### HttpMutationOptions \{#httpmutationoptions\}

Ƭ **HttpMutationOptions**: `Object`

#### 型宣言 \{#type-declaration\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `skipQueue` | `boolean` | デフォルトのミューテーションキューをスキップして、これをすぐに実行します。これにより、同じ HttpConvexClient で複数のミューテーションを並列にリクエストできるようになり、WebSocket ベースのクライアントではできなかったことが可能になります。 |

#### 定義元 \{#defined-in\}

[browser/http&#95;client.ts:40](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L40)

***

### ConvexClientOptions \{#convexclientoptions\}

Ƭ **ConvexClientOptions**: [`BaseConvexClientOptions`](../interfaces/browser.BaseConvexClientOptions.md) &amp; &#123; `disabled?`: `boolean` ; `unsavedChangesWarning?`: `boolean`  &#125;

#### 定義元 \{#defined-in\}

[browser/simple&#95;client.ts:36](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L36)

***

### AuthTokenFetcher \{#authtokenfetcher\}

Ƭ **AuthTokenFetcher**: (`args`: &#123; `forceRefreshToken`: `boolean`  &#125;) =&gt; `Promise`&lt;`string` | `null` | `undefined`&gt;

#### 型宣言 \{#type-declaration\}

▸ (`args`): `Promise`&lt;`string` | `null` | `undefined`&gt;

JWT を返す非同期関数です。convex/auth.config.ts で設定されている認証プロバイダーに応じて、JWT でエンコードされた OpenID Connect Identity Token か、従来の JWT のいずれかを返します。

`forceRefreshToken` は、サーバーが以前に返されたトークンを拒否した場合、または `exp` 時刻に基づいてトークンがまもなく有効期限切れになると予想される場合に `true` になります。

ConvexReactClient.setAuth を参照してください。

##### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `args` | `Object` |
| `args.forceRefreshToken` | `boolean` |

##### 戻り値 \{#returns\}

`Promise`&lt;`string` | `null` | `undefined`&gt;

#### 定義元 \{#defined-in\}

[browser/sync/authentication&#95;manager.ts:25](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/authentication_manager.ts#L25)

***

### ConnectionState \{#connectionstate\}

Ƭ **ConnectionState**: `Object`

クライアントと Convex バックエンド間の接続を表す状態。

#### 型宣言 \{#type-declaration\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `hasInflightRequests` | `boolean` | - |
| `isWebSocketConnected` | `boolean` | - |
| `timeOfOldestInflightRequest` | `Date` | `null` | - |
| `hasEverConnected` | `boolean` | クライアントが一度でも WebSocket を「ready」状態まで開いたことがある場合は `true`。 |
| `connectionCount` | `number` | このクライアントが Convex バックエンドに接続した回数。サーバーエラー、不安定なネットワーク接続、認証の有効期限切れなど、さまざまな要因でクライアントが再接続することがある。ただし、この数値が高い場合は、クライアントが安定した接続を維持できていない兆候となる。 |
| `connectionRetries` | `number` | このクライアントが Convex バックエンドへの接続を試みて（そして失敗して）きた回数。 |
| `inflightMutations` | `number` | 現在処理中のミューテーションの数。 |
| `inflightActions` | `number` | 現在処理中のアクションの数。 |

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:147](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L147)

***

### FunctionResult \{#functionresult\}

Ƭ **FunctionResult**: `FunctionSuccess` | `FunctionFailure`

サーバー側で関数を実行した結果を表します。

関数で例外が発生した場合は `errorMessage` を持ち、それ以外の場合は
`Value` を生成します。

#### 定義元 \{#defined-in\}

[browser/sync/function&#95;result.ts:11](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/function_result.ts#L11)

***

### OptimisticUpdate \{#optimisticupdate\}

Ƭ **OptimisticUpdate**&lt;`Args`&gt;: (`localQueryStore`: [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md), `args`: `Args`) =&gt; `void`

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Args` | extends `Record`&lt;`string`, [`Value`](values.md#value)&gt; |

#### 型宣言 \{#type-declaration\}

▸ (`localQueryStore`, `args`): `void`

このクライアント内のクエリ結果に対する、一時的かつローカルな更新です。

この更新は、ミューテーションが Convex サーバーへ同期されるときに必ず実行され、
ミューテーションが完了するとロールバックされます。

楽観的更新は複数回呼び出すことができる点に注意してください。
ミューテーションの処理中にクライアントが新しいデータを読み込んだ場合、
その更新は再度適用されます。

##### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `localQueryStore` | [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md) | ローカルのクエリ結果を読み取りや編集を行うためのインターフェース。 |
| `args` | `Args` | ミューテーションへの引数。 |

##### 戻り値 \{#returns\}

`void`

#### 定義元 \{#defined-in\}

[browser/sync/optimistic&#95;updates.ts:90](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/optimistic_updates.ts#L90)

***

### PaginationStatus \{#paginationstatus\}

Ƭ **PaginationStatus**: `"LoadingFirstPage"` | `"CanLoadMore"` | `"LoadingMore"` | `"Exhausted"`

#### 定義場所 \{#defined-in\}

[browser/sync/pagination.ts:5](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/pagination.ts#L5)

***

### QueryJournal \{#queryjournal\}

Ƭ **QueryJournal**: `string` | `null`

クエリの実行中に行われた決定内容をシリアライズした表現です。

ジャーナルは、クエリ関数が最初に実行されたときに生成され、クエリが再実行される際に再利用されます。

現在これは、ページネーション付きクエリの各ページが常に同じカーソルで終了することを保証するために、ページネーションの終了カーソルを保存する用途で使われています。これにより、ギャップのないリアクティブなページネーションが可能になります。

`null` は空のジャーナルを表すために使われます。

#### 定義元 \{#defined-in\}

[browser/sync/protocol.ts:113](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/protocol.ts#L113)

***

### QueryToken \{#querytoken\}

Ƭ **QueryToken**: `string` &amp; &#123; `__queryToken`: `true`  &#125;

クエリの名前と引数を表す文字列です。

この型は [BaseConvexClient](../classes/browser.BaseConvexClient.md) で使用されます。

#### 定義元 \{#defined-in\}

[browser/sync/udf&#95;path&#95;utils.ts:31](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/udf_path_utils.ts#L31)

***

### PaginatedQueryToken \{#paginatedquerytoken\}

Ƭ **PaginatedQueryToken**: [`QueryToken`](browser.md#querytoken) &amp; &#123; `__paginatedQueryToken`: `true`  &#125;

ページネーションされたクエリの名前と引数を表す文字列です。

これは、ページネーションされたクエリに使用される特殊な形式の QueryToken です。

#### 定義場所 \{#defined-in\}

[browser/sync/udf&#95;path&#95;utils.ts:38](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/udf_path_utils.ts#L38)

***

### UserIdentityAttributes \{#useridentityattributes\}

Ƭ **UserIdentityAttributes**: `Omit`&lt;[`UserIdentity`](../interfaces/server.UserIdentity.md), `"tokenIdentifier"`&gt;

#### 定義元 \{#defined-in\}

[server/authentication.ts:215](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L215)