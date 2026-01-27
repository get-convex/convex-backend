---
id: "react.ConvexReactClient"
title: "クラス: ConvexReactClient"
custom_edit_url: null
---

[react](../modules/react.md).ConvexReactClient

React 内で使用するための Convex クライアント。

このクライアントはリアクティブなクエリをロードし、WebSocket 経由でミューテーションを実行します。

## コンストラクタ \{#constructors\}

### コンストラクタ \{#constructor\}

• **new ConvexReactClient**(`address`, `options?`)

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `address` | `string` | Convex のデプロイメントの URL。多くの場合、環境変数から渡されます。例: `https://small-mouse-123.convex.cloud`。 |
| `options?` | [`ConvexReactClientOptions`](../interfaces/react.ConvexReactClientOptions.md) | 詳細については [ConvexReactClientOptions](../interfaces/react.ConvexReactClientOptions.md) を参照してください。 |

#### 定義場所 \{#defined-in\}

[react/client.ts:317](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L317)

## アクセサ \{#accessors\}

### url \{#url\}

• `get` **url**(): `string`

このクライアントのアドレスを返します。新しいクライアントを作成する際に便利です。

このクライアントが作成されたときに指定したアドレスと一致することは保証されません。正規化された形式に変換されている可能性があります。

#### 戻り値 \{#returns\}

`string`

#### 定義元 \{#defined-in\}

[react/client.ts:352](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L352)

***

### logger \{#logger\}

• `get` **logger**(): `Logger`

このクライアントのロガーを取得します。

#### 戻り値 \{#returns\}

`Logger`

このクライアントの Logger。

#### 定義元 \{#defined-in\}

[react/client.ts:713](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L713)

## メソッド \{#methods\}

### setAuth \{#setauth\}

▸ **setAuth**(`fetchToken`, `onChange?`): `void`

後続のクエリやミューテーションで使用される認証トークンを設定します。
`fetchToken` は、トークンの有効期限が切れた場合に自動的に再度呼び出されます。
`fetchToken` は、たとえばユーザーの権限が恒久的に失効した場合など、
トークンを取得できないときには `null` を返す必要があります。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `fetchToken` | [`AuthTokenFetcher`](../modules/browser.md#authtokenfetcher) | JWT でエンコードされた OpenID Connect の ID トークンを返す非同期関数 |
| `onChange?` | (`isAuthenticated`: `boolean`) =&gt; `void` | 認証状態が変化したときに呼び出されるコールバック |

#### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[react/client.ts:408](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L408)

***

### clearAuth \{#clearauth\}

▸ **clearAuth**(): `void`

現在設定されている認証トークンがあればクリアします。

#### 戻り値 \{#returns\}

`void`

#### 定義元 \{#defined-in\}

[react/client.ts:430](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L430)

***

### watchQuery \{#watchquery\}

▸ **watchQuery**&lt;`Query`&gt;(`query`, `...argsAndOptions`): [`Watch`](../interfaces/react.Watch.md)&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

Convex のクエリ関数に対する新しい [Watch](../interfaces/react.Watch.md) を作成します。

**ほとんどのアプリケーションコードでは、このメソッドを直接呼び出すべきではありません。代わりに
[useQuery](../modules/react.md#usequery) フックを使用してください。**

Watch を作成しても、その行為自体は何も行いません。Watch はステートレスです。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### パラメータ \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `query` | `Query` | 実行する公開クエリの [FunctionReference](../modules/server.md#functionreference)。 |
| `...argsAndOptions` | [`ArgsAndOptions`](../modules/server.md#argsandoptions)&lt;`Query`, [`WatchQueryOptions`](../interfaces/react.WatchQueryOptions.md)&gt; | - |

#### 戻り値 \{#returns\}

[`Watch`](../interfaces/react.Watch.md)&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

[`Watch`](../interfaces/react.Watch.md) オブジェクトを返します。

#### 定義場所 \{#defined-in\}

[react/client.ts:463](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L463)

***

### prewarmQuery \{#prewarmquery\}

▸ **prewarmQuery**&lt;`Query`&gt;(`queryOptions`): `void`

今後そのクエリを購読する可能性が高いことを示します。

現在の実装では、直ちにクエリを購読します。将来的には、このメソッドが一部のクエリを他より優先したり、購読せずにクエリ結果だけを取得したり、低速なネットワーク接続や高負荷の状況では何もしなかったりする可能性があります。

これを React コンポーネントで使う場合は、useQuery() を呼び出し、戻り値は無視してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `queryOptions` | `ConvexQueryOptions`&lt;`Query`&gt; &amp; &#123; `extendSubscriptionFor?`: `number`  &#125; | クエリ（api オブジェクトからの関数参照）とその引数に加え、そのクエリをどれくらいの期間購読するかを指定する任意の `extendSubscriptionFor` フィールド。 |

#### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[react/client.ts:539](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L539)

***

### mutation \{#mutation\}

▸ **mutation**&lt;`Mutation`&gt;(`mutation`, `...argsAndOptions`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

ミューテーション関数を実行します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | 実行するパブリックミューテーションを指定するための [FunctionReference](../modules/server.md#functionreference)。 |
| `...argsAndOptions` | [`ArgsAndOptions`](../modules/server.md#argsandoptions)&lt;`Mutation`, [`MutationOptions`](../interfaces/react.MutationOptions.md)&lt;[`FunctionArgs`](../modules/server.md#functionargs)&lt;`Mutation`&gt;&gt;&gt; | - |

#### 戻り値 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

ミューテーションの結果を返す Promise。

#### 定義場所 \{#defined-in\}

[react/client.ts:618](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L618)

***

### action \{#action\}

▸ **action**&lt;`Action`&gt;(`action`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

action 関数を実行します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"action"`&gt; |

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `action` | `Action` | 実行する公開アクションの [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Action`&gt; | アクションに渡す引数オブジェクト。省略した場合、引数は `{}` になります。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

アクションの結果を解決する Promise。

#### 定義元 \{#defined-in\}

[react/client.ts:639](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L639)

***

### query \{#query\}

▸ **query**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

クエリ結果を一度だけ取得します。

**ほとんどのアプリケーションコードでは、代わりに
[useQuery](../modules/react.md#usequery) フックを使用してクエリを購読してください。**

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | 実行する公開クエリを表す [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; | クエリに渡す引数オブジェクト。省略した場合、引数オブジェクトは `{}` になります。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

クエリ結果を返す `Promise`。

#### 定義場所 \{#defined-in\}

[react/client.ts:659](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L659)

***

### connectionState \{#connectionstate\}

▸ **connectionState**(): [`ConnectionState`](../modules/browser.md#connectionstate)

クライアントと Convex バックエンド間の現在の接続状態（[ConnectionState](../modules/browser.md#connectionstate)）を取得します。

#### 戻り値 \{#returns\}

[`ConnectionState`](../modules/browser.md#connectionstate)

Convex バックエンドとの接続状態を示す [ConnectionState](../modules/browser.md#connectionstate)。

#### 定義元 \{#defined-in\}

[react/client.ts:686](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L686)

***

### subscribeToConnectionState \{#subscribetoconnectionstate\}

▸ **subscribeToConnectionState**(`cb`): () =&gt; `void`

クライアントと Convex バックエンド間の[ConnectionState](../modules/browser.md#connectionstate)を購読し、
状態が変化するたびにコールバックを呼び出します。

購読されたコールバックは、ConnectionState のいずれかの部分が変更されたときに呼び出されます。
ConnectionState は将来のバージョンで拡張される可能性があります（たとえば、処理中（in‑flight）の
リクエストの配列を提供するなど）ため、その場合コールバックはより頻繁に呼び出されます。
また、どの情報が最も有用かが分かってくるにつれて、将来のバージョンでは ConnectionState の
プロパティが削除される可能性もあります。そのため、この API は不安定なものと見なされます。

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `cb` | (`connectionState`: [`ConnectionState`](../modules/browser.md#connectionstate)) =&gt; `void` |

#### 戻り値 \{#returns\}

`fn`

リッスンを停止するための購読解除関数。

▸ (): `void`

クライアントと Convex バックエンド間の [ConnectionState](../modules/browser.md#connectionstate) を購読し、
その状態が変化するたびにコールバックを呼び出します。

登録されたコールバックは、ConnectionState のいずれかの部分が変更されたときに呼び出されます。
ConnectionState は将来のバージョンで拡張される可能性があります（例: 処理中のリクエストの配列を提供するなど）。
その場合、コールバックはより頻繁に呼び出されることになります。
また、どの情報が最も有用かを判断する過程で、将来のバージョンでは ConnectionState からプロパティが *削除される* 可能性もあります。
そのため、この API は不安定なものと見なされます。

##### 戻り値 \{#returns\}

`void`

接続状態の購読を停止するための関数。

#### 定義場所 \{#defined-in\}

[react/client.ts:702](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L702)

***

### close \{#close\}

▸ **close**(): `Promise`&lt;`void`&gt;

このクライアントに関連するすべてのネットワークハンドルを閉じ、すべてのサブスクリプションを停止します。

[ConvexReactClient](react.ConvexReactClient.md) を使い終えたら、
そのソケットとリソースを解放するためにこのメソッドを呼び出してください。

#### Returns \{#returns\}

`Promise`&lt;`void`&gt;

接続が完全に閉じられたときに解決される `Promise`。

#### 定義場所 \{#defined-in\}

[react/client.ts:725](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L725)