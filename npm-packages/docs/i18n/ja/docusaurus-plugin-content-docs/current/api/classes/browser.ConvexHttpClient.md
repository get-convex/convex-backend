---
id: "browser.ConvexHttpClient"
title: "クラス: ConvexHttpClient"
custom_edit_url: null
---

[browser](../modules/browser.md).ConvexHttpClient

HTTP 経由でクエリおよびミューテーションを実行する Convex クライアントです。

このクライアントはステートフルです（ユーザーのクレデンシャルを保持し、ミューテーションをキューに追加します）。
そのため、サーバー内でリクエスト間で共有しないよう注意してください。

これはサーバーサイドのコード（Netlify の Lambda のようなもの）や、
リアクティブでない Web アプリケーションに適しています。

## コンストラクタ \{#constructors\}

### constructor \{#constructor\}

• **new ConvexHttpClient**(`address`, `options?`)

新しい [ConvexHttpClient](browser.ConvexHttpClient.md) インスタンスを作成します。

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `address` | `string` | Convex のデプロイメントURL。多くの場合は環境変数から渡されます。例: `https://small-mouse-123.convex.cloud`。 |
| `options?` | `Object` | オプションをまとめたオブジェクト。- `skipConvexDeploymentUrlCheck` - Convex のデプロイメントURL が `https://happy-animal-123.convex.cloud` または localhost のような形式かどうかの検証をスキップします。別の URL を使うセルフホスト型の Convex バックエンドを動かしている場合に便利です。- `logger` - logger もしくは boolean。指定しない場合はコンソールにログ出力します。独自の logger を作成して、別の出力先へログを送ったり、まったくログを出さないようにしたりできます。また、no-op logger の省略記法として `false` を使うこともできます。logger は 4 つのメソッド log(), warn(), error(), logVerbose() を持つオブジェクトです。これらのメソッドは console.log() と同様に、任意の型の複数の引数を受け取れます。- `auth` - Convex 関数から参照可能な identity claim を含む JWT。これは有効期限切れになる可能性があるため、後で `setAuth()` を呼び出す必要がある場合がありますが、短時間だけ動作するクライアントではここでこの値を指定しておくと便利です。- `fetch` - このクライアントが行うすべての HTTP リクエストに使用するカスタム fetch 実装。 |
| `options.skipConvexDeploymentUrlCheck?` | `boolean` | - |
| `options.logger?` | `boolean` | `Logger` | - |
| `options.auth?` | `string` | - |
| `options.fetch?` | (`input`: `URL` | `RequestInfo`, `init?`: `RequestInit`) =&gt; `Promise`&lt;`Response`&gt;(`input`: `string` | `URL` | `Request`, `init?`: `RequestInit`) =&gt; `Promise`&lt;`Response`&gt; | - |

#### 定義元 \{#defined-in\}

[browser/http&#95;client.ts:97](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L97)

## アクセサ \{#accessors\}

### url \{#url\}

• `get` **url**(): `string`

このクライアント用のアドレスを返します。新しいクライアントを作成する際に便利です。

このクライアントが生成されたときに指定したアドレスと一致することは保証されません。
正規的な形式に正規化されている可能性があります。

#### 戻り値 \{#returns\}

`string`

#### 定義元 \{#defined-in\}

[browser/http&#95;client.ts:147](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L147)

## メソッド \{#methods\}

### backendUrl \{#backendurl\}

▸ **backendUrl**(): `string`

[ConvexHttpClient](browser.ConvexHttpClient.md) が接続しているバックエンドの URL を取得します。

**`Deprecated`**

代わりに、末尾に /api を含まない URL を返す url を使用してください。

#### 戻り値 \{#returns\}

`string`

クライアントの API バージョンを含む Convex バックエンドの URL。

#### 定義元 \{#defined-in\}

[browser/http&#95;client.ts:137](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L137)

***

### setAuth \{#setauth\}

▸ **setAuth**(`value`): `void`

以降のクエリやミューテーションで使用する認証トークンを設定します。

トークンが有効期限切れやリフレッシュなどで変更されたときには、その都度必ず呼び出してください。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `value` | `string` | JWT でエンコードされた OpenID Connect の ID トークン。 |

#### 戻り値 \{#returns\}

`void`

#### 定義元 \{#defined-in\}

[browser/http&#95;client.ts:158](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L158)

***

### clearAuth \{#clearauth\}

▸ **clearAuth**(): `void`

現在の認証トークンをクリアします。

#### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[browser/http&#95;client.ts:184](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L184)

***

### consistentQuery \{#consistentquery\}

▸ **consistentQuery**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

この API は実験的なものであり、将来変更されたり削除されたりする可能性があります。

Convex のクエリ関数を、この HTTP クライアントによって実行される
他のすべての一貫性のあるクエリ実行と同じタイムスタンプで実行します。

Convex バックエンドは過去のデータを読める範囲が限られており、
30 秒以上前の状態は利用できない可能性があるため、
長寿命の ConvexHttpClient では意味がありません。

同一のタイムスタンプを使うには、新しいクライアントを作成してください。

**`Deprecated`**

この API は実験的なものであり、将来変更されたり削除されたりする可能性があります。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | - |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; | クエリの引数オブジェクトです。このパラメータを省略した場合、引数は `{}` になります。 |

#### Returns \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

クエリ結果を返す Promise。

#### 定義場所 \{#defined-in\}

[browser/http&#95;client.ts:226](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L226)

***

### query \{#query\}

▸ **query**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

Convex のクエリ関数を実行します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | - |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; | クエリの引数オブジェクトです。省略された場合、引数は `{}` になります。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

クエリ結果を返す Promise。

#### 定義場所 \{#defined-in\}

[browser/http&#95;client.ts:270](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L270)

***

### ミューテーション \{#mutation\}

▸ **mutation**&lt;`Mutation`&gt;(`mutation`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

Convex のミューテーション関数を実行します。ミューテーションはデフォルトでキューに入れられます。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | - |
| `...args` | [`ArgsAndOptions`](../modules/server.md#argsandoptions)&lt;`Mutation`, [`HttpMutationOptions`](../modules/browser.md#httpmutationoptions)&gt; | ミューテーションに渡す引数オブジェクト。これを省略した場合、引数は `{}` になります。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

ミューテーション結果を返す Promise。

#### 定義元 \{#defined-in\}

[browser/http&#95;client.ts:430](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L430)

***

### action \{#action\}

▸ **action**&lt;`Action`&gt;(`action`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

Convex のアクション関数を実行します。アクションはキューに追加されません。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"action"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `action` | `Action` | - |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Action`&gt; | アクションの引数オブジェクト。これを省略すると、引数は `{}` になります。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

アクションの結果を返す `Promise`。

#### 定義元 \{#defined-in\}

[browser/http&#95;client.ts:453](https://github.com/get-convex/convex-js/blob/main/src/browser/http_client.ts#L453)