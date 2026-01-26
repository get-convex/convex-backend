---
id: "browser.BaseConvexClientOptions"
title: "インターフェース: BaseConvexClientOptions"
custom_edit_url: null
---

[browser](../modules/browser.md).BaseConvexClientOptions

[BaseConvexClient](../classes/browser.BaseConvexClient.md) 用のオプション。

## 継承階層 \{#hierarchy\}

* **`BaseConvexClientOptions`**

  ↳ [`ConvexReactClientOptions`](react.ConvexReactClientOptions.md)

## プロパティ \{#properties\}

### unsavedChangesWarning \{#unsavedchangeswarning\}

• `Optional` **unsavedChangesWarning**: `boolean`

ページから移動したり、ウェブページを閉じたりする際に、
未保存の変更がある場合にユーザーに確認を促すかどうかを指定します。

これは `window` オブジェクトが存在する場合、つまりブラウザ上でのみ有効です。

ブラウザでのデフォルト値は `true` です。

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:69](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L69)

***

### webSocketConstructor \{#websocketconstructor\}

• `Optional` **webSocketConstructor**: `Object`

#### 呼び出しシグネチャ \{#call-signature\}

• **new webSocketConstructor**(`url`, `protocols?`): `WebSocket`

クライアントが Convex クラウドと通信する際に使用する代替の
[WebSocket](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)
コンストラクタを指定します。
既定では、グローバル環境の `WebSocket` が使用されます。

##### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `url` | `string` | `URL` |
| `protocols?` | `string` | `string`[] |

##### 戻り値 \{#returns\}

`WebSocket`

#### 型定義 \{#type-declaration\}

| 名前 | 型 |
| :------ | :------ |
| `prototype` | `WebSocket` |
| `CONNECTING` | `0` |
| `OPEN` | `1` |
| `CLOSING` | `2` |
| `CLOSED` | `3` |

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:76](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L76)

***

### verbose \{#verbose\}

• `Optional` **verbose**: `boolean`

デバッグ用にログ出力を追加します。

デフォルト値は `false` です。

#### 定義場所 \{#defined-in\}

[browser/sync/client.ts:82](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L82)

***

### logger \{#logger\}

• `Optional` **logger**: `boolean` | `Logger`

logger、`true`、または `false` のいずれかです。指定しない場合、または `true` の場合はコンソールにログを出力します。
`false` の場合、ログはどこにも出力されません。

独自の logger を作成して、別の場所へのログ出力などをカスタマイズできます。
logger は log(), warn(), error(), logVerbose() の 4 つのメソッドを持つオブジェクトです。
これらのメソッドは、console.log() と同様に、任意の型の複数の引数を受け取ることができます。

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:91](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L91)

***

### reportDebugInfoToConvex \{#reportdebuginfotoconvex\}

• `Optional` **reportDebugInfoToConvex**: `boolean`

デバッグのために、追加のメトリクスを Convex に送信します。

デフォルト値は `false` です。

#### 定義場所 \{#defined-in\}

[browser/sync/client.ts:97](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L97)

***

### onServerDisconnectError \{#onserverdisconnecterror\}

• `省略可` **onServerDisconnectError**: (`message`: `string`) =&gt; `void`

#### 型定義 \{#type-declaration\}

▸ (`message`): `void`

この API は実験的です。将来、変更されたり廃止されたりする可能性があります。

接続中の Convex デプロイメントから、異常な WebSocket のクローズメッセージを受信した際に呼び出される関数です。これらのメッセージの内容は安定しておらず、実装上の詳細であり今後変更される可能性があります。

この API は、推奨される対処方法付きの、より高レベルなコード（エラーコードなど）が提供されるまでの間の、オブザーバビリティのための暫定的な手段だと考えてください。そのようなコードは、`string` の代わりに、より安定したインターフェースを提供するかもしれません。

接続状態に関するより定量的なメトリクスについては、`connectionState` を確認してください。

##### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `message` | `string` |

##### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[browser/sync/client.ts:111](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L111)

***

### skipConvexDeploymentUrlCheck \{#skipconvexdeploymenturlcheck\}

• `Optional` **skipConvexDeploymentUrlCheck**: `boolean`

Convex のデプロイメントURL が
`https://happy-animal-123.convex.cloud` または localhost の形式になっているかどうかの検証をスキップします。

別の URL を使用するセルフホスト型の Convex バックエンドを実行している場合に便利です。

デフォルト値は `false` です。

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:121](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L121)

***

### authRefreshTokenLeewaySeconds \{#authrefreshtokenleewayseconds\}

• `Optional` **authRefreshTokenLeewaySeconds**: `number`

認証を使用している場合に、トークンの有効期限が切れる何秒前にリフレッシュするかを指定します。

デフォルト値は `2` です。

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:127](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L127)

***

### expectAuth \{#expectauth\}

• `Optional` **expectAuth**: `boolean`

この API は実験的です。将来的に変更されたり廃止されたりする可能性があります。

最初の認証トークンを送信できるようになるまで、
クエリ、ミューテーション、アクションのリクエストを保留するかどうかを指定します。

この挙動を有効にすると、認証済みクライアントだけが
閲覧できるべきページでうまく機能します。

デフォルトは `false` で、認証トークンを待ちません。

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:139](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L139)