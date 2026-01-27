---
id: "browser.BaseConvexClient"
title: "クラス: BaseConvexClient"
custom_edit_url: null
---

[browser](../modules/browser.md).BaseConvexClient

状態管理ライブラリを Convex と直接統合するための低レベルのクライアントです。

多くの開発者には、[ConvexHttpClient](browser.ConvexHttpClient.md) や React のフックベースの [ConvexReactClient](react.ConvexReactClient.md) といった、より高レベルなクライアントの利用が推奨されます。

## コンストラクター \{#constructors\}

### コンストラクタ \{#constructor\}

• **new BaseConvexClient**(`address`, `onTransition`, `options?`)

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `address` | `string` | Convex デプロイメントの URL。多くの場合、環境変数から指定されます。例: `https://small-mouse-123.convex.cloud`。 |
| `onTransition` | (`updatedQueries`: [`QueryToken`](../modules/browser.md#querytoken)[]) =&gt; `void` | 変更されたクエリ結果に対応するクエリトークンの配列を受け取るコールバック関数です。追加のハンドラーは `addOnTransitionHandler` で登録できます。 |
| `options?` | [`BaseConvexClientOptions`](../interfaces/browser.BaseConvexClientOptions.md) | 詳細な説明は [BaseConvexClientOptions](../interfaces/browser.BaseConvexClientOptions.md) を参照してください。 |

#### 定義場所 \{#defined-in\}

[browser/sync/client.ts:277](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L277)

## アクセサ \{#accessors\}

### url \{#url\}

• `get` **url**(): `string`

このクライアントのアドレスを返します。新しいクライアントを作成する際に役立ちます。

このクライアントが生成されたときのアドレスと一致することは保証されません。
正規化された形式に変換されている可能性があります。

#### 戻り値 \{#returns\}

`string`

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:1037](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L1037)

## メソッド \{#methods\}

### getMaxObservedTimestamp \{#getmaxobservedtimestamp\}

▸ **getMaxObservedTimestamp**(): `undefined` | `Long`

#### 戻り値 \{#returns\}

`undefined` | `Long`

#### 定義場所 \{#defined-in\}

[browser/sync/client.ts:542](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L542)

***

### addOnTransitionHandler \{#addontransitionhandler\}

▸ **addOnTransitionHandler**(`fn`): () =&gt; `boolean`

トランジション時に呼び出されるハンドラーを追加します。

外部への副作用（例: React の state を更新する処理）はここで行ってください。

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `fn` | (`transition`: `Transition`) =&gt; `void` |

#### 戻り値 \{#returns\}

`fn`

▸ (): `boolean`

##### 戻り値 \{#returns\}

`boolean`

#### 定義場所 \{#defined-in\}

[browser/sync/client.ts:621](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L621)

***

### getCurrentAuthClaims \{#getcurrentauthclaims\}

▸ **getCurrentAuthClaims**(): `undefined` | &#123; `token`: `string` ; `decoded`: `Record`&lt;`string`, `any`&gt;  &#125;

現在の JWT 認証トークンとデコードされたクレームを取得します。

#### 戻り値 \{#returns\}

`undefined` | &#123; `token`: `string` ; `decoded`: `Record`&lt;`string`, `any`&gt;  &#125;

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:630](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L630)

***

### setAuth \{#setauth\}

▸ **setAuth**(`fetchToken`, `onChange`): `void`

以降のクエリとミューテーションで使用される認証トークンを設定します。
トークンの有効期限が切れた場合、`fetchToken` は自動的に再度呼び出されます。
ユーザーの権限が完全に取り消された場合など、トークンを取得できないときは、
`fetchToken` は `null` を返す必要があります。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `fetchToken` | [`AuthTokenFetcher`](../modules/browser.md#authtokenfetcher) | JWT 形式でエンコードされた OpenID Connect の ID トークンを返す非同期関数 |
| `onChange` | (`isAuthenticated`: `boolean`) =&gt; `void` | 認証状態が変化したときに呼び出されるコールバック |

#### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[browser/sync/client.ts:655](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L655)

***

### hasAuth \{#hasauth\}

▸ **hasAuth**(): `boolean`

#### 戻り値 \{#returns\}

`boolean`

#### 定義場所 \{#defined-in\}

[browser/sync/client.ts:662](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L662)

***

### clearAuth \{#clearauth\}

▸ **clearAuth**(): `void`

#### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[browser/sync/client.ts:672](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L672)

***

### subscribe \{#subscribe\}

▸ **subscribe**(`name`, `args?`, `options?`): `Object`

クエリ関数を購読します。

このクエリの結果が変更されるたびに、コンストラクタに渡された `onTransition` コールバックが呼び出されます。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `string` | クエリの名前。 |
| `args?` | `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; | クエリに渡す引数オブジェクト。省略した場合、引数は `{}` になります。 |
| `options?` | [`SubscribeOptions`](../interfaces/browser.SubscribeOptions.md) | このクエリ用の [SubscribeOptions](../interfaces/browser.SubscribeOptions.md) オプションオブジェクト。 |

#### 戻り値 \{#returns\}

`Object`

このクエリに対応する [QueryToken](../modules/browser.md#querytoken) と、`unsubscribe` コールバックを含むオブジェクトです。

| Name | Type |
| :------ | :------ |
| `queryToken` | [`QueryToken`](../modules/browser.md#querytoken) |
| `unsubscribe` | () =&gt; `void` |

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:691](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L691)

***

### localQueryResult \{#localqueryresult\}

▸ **localQueryResult**(`udfPath`, `args?`): `undefined` | [`Value`](../modules/values.md#value)

現在のローカル状態のみに基づくクエリ結果です。

値が返されるのは、そのクエリをすでに購読している場合か、そのクエリの値が楽観的に設定されている場合に限られます。

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `udfPath` | `string` |
| `args?` | `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; |

#### 戻り値 \{#returns\}

`undefined` | [`Value`](../modules/values.md#value)

#### 定義場所 \{#defined-in\}

[browser/sync/client.ts:724](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L724)

***

### queryJournal \{#queryjournal\}

▸ **queryJournal**(`name`, `args?`): `undefined` | [`QueryJournal`](../modules/browser.md#queryjournal)

このクエリ関数に対する現在の [QueryJournal](../modules/browser.md#queryjournal) を取得します。

このクエリの結果をまだ受け取っていない場合、返り値は `undefined` になります。

#### パラメータ \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `name` | `string` | クエリの名前。 |
| `args?` | `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; | このクエリの引数を表すオブジェクト。 |

#### 戻り値 \{#returns\}

`undefined` | [`QueryJournal`](../modules/browser.md#queryjournal)

クエリの [QueryJournal](../modules/browser.md#queryjournal)、または `undefined`。

#### 定義場所 \{#defined-in\}

[browser/sync/client.ts:777](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L777)

***

### connectionState \{#connectionstate\}

▸ **connectionState**(): [`ConnectionState`](../modules/browser.md#connectionstate)

クライアントと Convex バックエンド間の現在の [ConnectionState](../modules/browser.md#connectionstate) を取得します。

#### 戻り値 \{#returns\}

[`ConnectionState`](../modules/browser.md#connectionstate)

Convex バックエンドとの接続状態を表す [ConnectionState](../modules/browser.md#connectionstate)。

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:792](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L792)

***

### subscribeToConnectionState \{#subscribetoconnectionstate\}

▸ **subscribeToConnectionState**(`cb`): () =&gt; `void`

クライアントと Convex バックエンド間の[ConnectionState](../modules/browser.md#connectionstate)を購読し、状態が変化するたびにコールバックを呼び出します。

購読されたコールバックは、ConnectionState のいずれかの部分が変更されたときに呼び出されます。
ConnectionState は将来のバージョンで拡張される可能性があります（例: 進行中のリクエストの配列を提供するなど）。その場合、コールバックはより頻繁に呼び出されることになります。

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `cb` | (`connectionState`: [`ConnectionState`](../modules/browser.md#connectionstate)) =&gt; `void` |

#### 返り値 \{#returns\}

`fn`

リッスンを停止するための購読解除用関数。

▸ (): `void`

クライアントと Convex バックエンド間の [ConnectionState](../modules/browser.md#connectionstate) を購読し、状態が変化するたびにコールバックを呼び出します。

登録されたコールバックは、ConnectionState のいずれかの部分が変更されたときに呼び出されます。
ConnectionState は将来のバージョンで拡張される可能性があります（例: 送信中のリクエストの配列を提供するなど）。その場合、コールバックはより頻繁に呼び出されるようになります。

##### 戻り値 \{#returns\}

`void`

購読を停止するための関数。

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:838](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L838)

***

### ミューテーション \{#mutation\}

▸ **mutation**(`name`, `args?`, `options?`): `Promise`&lt;`any`&gt;

ミューテーションを実行します。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `string` | ミューテーションの名前。 |
| `args?` | `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; | ミューテーションの引数オブジェクト。省略した場合、引数は `{}` になります。 |
| `options?` | [`MutationOptions`](../interfaces/browser.MutationOptions.md) | このミューテーション用の [MutationOptions](../interfaces/browser.MutationOptions.md) オブジェクト。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`any`&gt;

* ミューテーションの結果を返す `Promise`。

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:858](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L858)

***

### action \{#action\}

▸ **action**(`name`, `args?`): `Promise`&lt;`any`&gt;

アクション関数を実行します。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `string` | アクションの名前。 |
| `args?` | `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; | アクションの引数オブジェクト。このパラメーターを省略すると、引数は `{}` になります。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`any`&gt;

アクションの結果を解決する Promise。

#### 定義場所 \{#defined-in\}

[browser/sync/client.ts:979](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L979)

***

### close \{#close\}

▸ **close**(): `Promise`&lt;`void`&gt;

このクライアントに関連付けられているすべてのネットワークハンドルを閉じ、すべてのサブスクリプションを停止します。

[BaseConvexClient](browser.BaseConvexClient.md) の使用が終わったら、このメソッドを呼び出して
ソケットおよびリソースを破棄してください。

#### Returns \{#returns\}

`Promise`&lt;`void`&gt;

接続が完全に閉じられたときに解決される`Promise`。

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:1026](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L1026)