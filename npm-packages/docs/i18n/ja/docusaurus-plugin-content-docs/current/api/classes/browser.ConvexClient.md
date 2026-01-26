---
id: "browser.ConvexClient"
title: "クラス: ConvexClient"
custom_edit_url: null
---

[browser](../modules/browser.md).ConvexClient

Convex のクエリ関数を購読し、WebSocket 経由でミューテーションとアクションを実行します。

このクライアントでは、ミューテーションに対する楽観的更新はサポートされていません。
サードパーティクライアントは、より細かく制御するために [BaseConvexClient](browser.BaseConvexClient.md) をラップすることができます。

```ts
const client = new ConvexClient("https://happy-otter-123.convex.cloud");
const unsubscribe = client.onUpdate(api.messages.list, {}, (messages) => {
  console.log(messages[0].body);
});
```

## コンストラクタ \{#constructors\}

### constructor \{#constructor\}

• **new ConvexClient**(`address`, `options?`)

クライアントを作成し、渡されたアドレスへの WebSocket 接続をすぐに開始します。

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `address` | `string` |
| `options` | [`ConvexClientOptions`](../modules/browser.md#convexclientoptions) |

#### 定義場所 \{#defined-in\}

[browser/simple&#95;client.ts:119](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L119)

## アクセサー \{#accessors\}

### closed \{#closed\}

• `get` **closed**(): `boolean`

一度クローズされると、登録済みのコールバックが再度呼び出されることはありません。

#### 戻り値 \{#returns\}

`boolean`

#### 定義場所 \{#defined-in\}

[browser/simple&#95;client.ts:96](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L96)

***

### client \{#client\}

• `get` **client**(): [`BaseConvexClient`](browser.BaseConvexClient.md)

#### 戻り値 \{#returns\}

[`BaseConvexClient`](browser.BaseConvexClient.md)

#### 定義元 \{#defined-in\}

[browser/simple&#95;client.ts:99](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L99)

***

### disabled \{#disabled\}

• `get` **disabled**(): `boolean`

#### 戻り値 \{#returns\}

`boolean`

#### 定義元 \{#defined-in\}

[browser/simple&#95;client.ts:110](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L110)

## メソッド \{#methods\}

### onUpdate \{#onupdate\}

▸ **onUpdate**&lt;`Query`&gt;(`query`, `args`, `callback`, `onError?`): `Unsubscribe`&lt;`Query`[`"_returnType"`]&gt;

クエリの新しい結果を受信するたびに、コールバックを呼び出します。クエリの結果がすでに
メモリ上に存在する場合は、登録後すぐにコールバックが実行されます。

戻り値は Unsubscribe オブジェクトで、関数であると同時にプロパティも持っています。
このオブジェクトには、次のどちらのパターンも利用できます:

```ts
// call the return value as a function
const unsubscribe = client.onUpdate(api.messages.list, {}, (messages) => {
  console.log(messages);
});
unsubscribe();

// 戻り値をそのプロパティに分割代入する
const {
  getCurrentValue,
  unsubscribe,
} = client.onUpdate(api.messages.list, {}, (messages) => {
  console.log(messages);
});
```

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | 実行する公開クエリの [FunctionReference](../modules/server.md#functionreference)。 |
| `args` | [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Query`&gt; | クエリの実行に使用する引数。 |
| `callback` | (`result`: [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;) =&gt; `unknown` | クエリ結果が更新されたときに呼び出す関数。 |
| `onError?` | (`e`: `Error`) =&gt; `unknown` | クエリ結果の更新時にエラーが発生した場合に呼び出す関数。指定しない場合、コールバックを呼び出す代わりにエラーがスローされます。 |

#### 戻り値 \{#returns\}

`Unsubscribe`&lt;`Query`[`"_returnType"`]&gt;

`onUpdate` 関数の呼び出しを停止するための `Unsubscribe` 関数。

#### 定義場所 \{#defined-in\}

[browser/simple&#95;client.ts:185](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L185)

***

### onPaginatedUpdate_experimental \{#onpaginatedupdate_experimental\}

▸ **onPaginatedUpdate&#95;experimental**&lt;`Query`&gt;(`query`, `args`, `options`, `callback`, `onError?`): `Unsubscribe`&lt;`PaginatedQueryResult`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;[]&gt;&gt;

ページネーションされたクエリで新しい結果が受信されるたびに、コールバックを呼び出します。

これは実験的なプレビュー機能です。最終的な API は変更される可能性があります。
特に、キャッシュの挙動、ページ分割、および必須となるページネーション付きクエリのオプションは
変更される可能性があります。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | 実行する公開クエリを指定する [FunctionReference](../modules/server.md#functionreference)。 |
| `args` | [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Query`&gt; | このクエリを実行する際に渡す引数。 |
| `options` | `Object` | `initialNumItems` と `id` を含む、ページネーションされたクエリ用のオプション。 |
| `options.initialNumItems` | `number` | - |
| `callback` | (`result`: [`PaginationResult`](../interfaces/server.PaginationResult.md)&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;) =&gt; `unknown` | クエリ結果が更新されたときに呼び出される関数。 |
| `onError?` | (`e`: `Error`) =&gt; `unknown` | クエリ結果がエラーで更新されたときに呼び出される関数。 |

#### Returns \{#returns\}

`Unsubscribe`&lt;`PaginatedQueryResult`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;[]&gt;&gt;

コールバックの呼び出しを停止するための Unsubscribe 関数です。

#### 定義場所 \{#defined-in\}

[browser/simple&#95;client.ts:263](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L263)

***

### close \{#close\}

▸ **close**(): `Promise`&lt;`void`&gt;

#### 戻り値 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定義場所 \{#defined-in\}

[browser/simple&#95;client.ts:366](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L366)

***

### getAuth \{#getauth\}

▸ **getAuth**(): `undefined` | &#123; `token`: `string` ; `decoded`: `Record`&lt;`string`, `any`&gt;  &#125;

現在の JWT 認証トークンおよびそのデコード済みクレームを取得します。

#### 戻り値 \{#returns\}

`undefined` | &#123; `token`: `string` ; `decoded`: `Record`&lt;`string`, `any`&gt;  &#125;

#### 定義元 \{#defined-in\}

[browser/simple&#95;client.ts:380](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L380)

***

### setAuth \{#setauth\}

▸ **setAuth**(`fetchToken`, `onChange?`): `void`

後続のクエリおよびミューテーションで使用する認証トークンを設定します。
`fetchToken` は、トークンの有効期限が切れた場合に自動的に再度呼び出されます。
`fetchToken` は、たとえばユーザーの権限が完全に取り消された場合など、
トークンを取得できないときには `null` を返す必要があります。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `fetchToken` | [`AuthTokenFetcher`](../modules/browser.md#authtokenfetcher) | JWT（通常は OpenID Connect の ID トークン）を返す非同期関数 |
| `onChange?` | (`isAuthenticated`: `boolean`) =&gt; `void` | 認証状態が変化したときに呼び出されるコールバック |

#### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[browser/simple&#95;client.ts:393](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L393)

***

### ミューテーション \{#mutation\}

▸ **mutation**&lt;`Mutation`&gt;(`mutation`, `args`, `options?`): `Promise`&lt;`Awaited`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;&gt;

ミューテーション関数を実行します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`&gt; |

#### パラメーター \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | 実行する公開ミューテーションの [FunctionReference](../modules/server.md#functionreference)。 |
| `args` | [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Mutation`&gt; | ミューテーションに渡す引数オブジェクト。 |
| `options?` | [`MutationOptions`](../interfaces/browser.MutationOptions.md) | ミューテーション用の [MutationOptions](../interfaces/browser.MutationOptions.md) 設定オブジェクト。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`Awaited`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;&gt;

ミューテーションの結果に解決される `Promise`。

#### 定義場所 \{#defined-in\}

[browser/simple&#95;client.ts:488](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L488)

***

### action \{#action\}

▸ **action**&lt;`Action`&gt;(`action`, `args`): `Promise`&lt;`Awaited`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;&gt;

アクション関数を実行します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"action"`&gt; |

#### パラメータ \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `action` | `Action` | 実行する公開アクションを指す [FunctionReference](../modules/server.md#functionreference)。 |
| `args` | [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Action`&gt; | アクションに渡す引数オブジェクト。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`Awaited`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;&gt;

アクションの結果が解決される Promise。

#### 定義元 \{#defined-in\}

[browser/simple&#95;client.ts:505](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L505)

***

### query \{#query\}

▸ **query**&lt;`Query`&gt;(`query`, `args`): `Promise`&lt;`Awaited`&lt;`Query`[`"_returnType"`]&gt;&gt;

クエリ結果を一度だけ取得します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | 実行する公開クエリを指す [FunctionReference](../modules/server.md#functionreference)。 |
| `args` | `Query`[`"_args"`] | クエリ用の引数オブジェクト。 |

#### Returns \{#returns\}

`Promise`&lt;`Awaited`&lt;`Query`[`"_returnType"`]&gt;&gt;

クエリの結果を返す Promise。

#### 定義場所 \{#defined-in\}

[browser/simple&#95;client.ts:521](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L521)

***

### connectionState \{#connectionstate\}

▸ **connectionState**(): [`ConnectionState`](../modules/browser.md#connectionstate)

クライアントと Convex バックエンド間の現在の接続状態（[`ConnectionState`](../modules/browser.md#connectionstate)）を取得します。

#### 戻り値 \{#returns\}

[`ConnectionState`](../modules/browser.md#connectionstate)

Convex バックエンドとの接続状態を表す [`ConnectionState`](../modules/browser.md#connectionstate)。

#### 定義元 \{#defined-in\}

[browser/simple&#95;client.ts:553](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L553)

***

### subscribeToConnectionState \{#subscribetoconnectionstate\}

▸ **subscribeToConnectionState**(`cb`): () =&gt; `void`

クライアントと Convex バックエンド間の [ConnectionState](../modules/browser.md#connectionstate) を購読し、
状態が変化するたびにコールバックを呼び出します。

購読されたコールバックは、ConnectionState のいずれかの部分が変更されるたびに呼び出されます。
将来のバージョンでは、たとえばインフライト中のリクエストの配列を提供するなど、ConnectionState が拡張される可能性があります。
その場合、コールバックが呼び出される頻度も高くなります。

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `cb` | (`connectionState`: [`ConnectionState`](../modules/browser.md#connectionstate)) =&gt; `void` |

#### 戻り値 \{#returns\}

`fn`

リッスンを停止するための購読解除関数。

▸ (): `void`

クライアントと Convex バックエンド間の [ConnectionState](../modules/browser.md#connectionstate) を購読し、状態が変化するたびにコールバックを呼び出します。

登録されたコールバックは、ConnectionState のいずれかの部分が変化したときに呼び出されます。
ConnectionState は将来のバージョンで拡張される可能性があります（例: 処理中のリクエストの配列を提供するなど）。その場合、コールバックはより頻繁に呼び出されるようになります。

##### 戻り値 \{#returns\}

`void`

リッスンを停止するための購読解除用関数。

#### 定義元 \{#defined-in\}

[browser/simple&#95;client.ts:568](https://github.com/get-convex/convex-js/blob/main/src/browser/simple_client.ts#L568)