---
id: "react.ConvexReactClientOptions"
title: "インターフェース: ConvexReactClientOptions"
custom_edit_url: null
---

[react](../modules/react.md).ConvexReactClientOptions

[ConvexReactClient](../classes/react.ConvexReactClient.md) 用のオプション。

## 継承階層 \{#hierarchy\}

* [`BaseConvexClientOptions`](browser.BaseConvexClientOptions.md)

  ↳ **`ConvexReactClientOptions`**

## プロパティ \{#properties\}

### unsavedChangesWarning \{#unsavedchangeswarning\}

• `Optional` **unsavedChangesWarning**: `boolean`

未保存の変更がある状態でページから離れたり Web ページを閉じたりする際に、ユーザーに確認を促すかどうか。

これは `window` オブジェクトが存在する場合、つまりブラウザ上でのみ有効です。

ブラウザでのデフォルト値は `true` です。

#### 継承元 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[unsavedChangesWarning](browser.BaseConvexClientOptions.md#unsavedchangeswarning)

#### 定義先 \{#defined-in\}

[browser/sync/client.ts:69](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L69)

***

### webSocketConstructor \{#websocketconstructor\}

• `Optional` **webSocketConstructor**: `Object`

#### 呼び出しシグネチャ \{#call-signature\}

• **new webSocketConstructor**(`url`, `protocols?`): `WebSocket`

Convex クラウドとのクライアント通信に使用する
[WebSocket](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)
コンストラクタとして別のものを指定します。
既定では、グローバル環境の `WebSocket` が使用されます。

##### パラメーター \{#parameters\}

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

#### 継承元 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[webSocketConstructor](browser.BaseConvexClientOptions.md#websocketconstructor)

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:76](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L76)

***

### verbose \{#verbose\}

• `Optional` **verbose**: `boolean`

デバッグ目的で追加のログ出力を有効にします。

デフォルト値は `false` です。

#### 継承元 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[verbose](browser.BaseConvexClientOptions.md#verbose)

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:82](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L82)

***

### logger \{#logger\}

• `Optional` **logger**: `boolean` | `Logger`

logger には、`true`、`false`、または `logger` を指定します。指定しない場合、または `true` の場合はコンソールにログを出力します。
`false` の場合、ログはどこにも出力されません。

独自の logger を作成して、別の出力先へのロギングなどをカスタマイズできます。
logger は `log()`、`warn()`、`error()`、`logVerbose()` の 4 つのメソッドを持つオブジェクトです。
これらのメソッドは、`console.log()` と同様に、任意の型の複数の引数を受け取ることができます。

#### 継承元 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[logger](browser.BaseConvexClientOptions.md#logger)

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:91](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L91)

***

### reportDebugInfoToConvex \{#reportdebuginfotoconvex\}

• `Optional` **reportDebugInfoToConvex**: `boolean`

デバッグのために追加のメトリクスを Convex に送信するかどうかを指定します。

デフォルト値は `false` です。

#### 継承元 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[reportDebugInfoToConvex](browser.BaseConvexClientOptions.md#reportdebuginfotoconvex)

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:97](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L97)

***

### onServerDisconnectError \{#onserverdisconnecterror\}

• `Optional` **onServerDisconnectError**: (`message`: `string`) =&gt; `void`

#### 型宣言 \{#type-declaration\}

▸ (`message`): `void`

この API は試験的機能です。今後、変更されたり削除されたりする可能性があります。

接続中の Convex デプロイメントから、異常な WebSocket クローズメッセージを受信したときに呼び出される関数です。これらのメッセージの内容は安定しておらず、実装の詳細であり、今後の変更によって変わる可能性があります。

この API は、より高レベルなコード体系と、それに基づく「何をすべきか」の推奨事項が提供されるまでの間の、可観測性向上のための一時的な手段と考えてください。将来的には、`string` よりも安定したインターフェースに置き換えられる可能性があります。

接続状態に関するより定量的なメトリクスについては、`connectionState` を確認してください。

##### パラメータ \{#parameters\}

| パラメータ名 | 型 |
| :------ | :------ |
| `message` | `string` |

##### 戻り値 \{#returns\}

`void`

#### 継承元 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[onServerDisconnectError](browser.BaseConvexClientOptions.md#onserverdisconnecterror)

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:111](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L111)

***

### skipConvexDeploymentUrlCheck \{#skipconvexdeploymenturlcheck\}

• `Optional` **skipConvexDeploymentUrlCheck**: `boolean`

Convex のデプロイメントURL が
`https://happy-animal-123.convex.cloud` または localhost のような形式かどうかを検証する処理をスキップします。

別の URL を使用するセルフホスト型の Convex バックエンドを実行している場合に便利です。

デフォルト値は `false` です

#### 継承元 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[skipConvexDeploymentUrlCheck](browser.BaseConvexClientOptions.md#skipconvexdeploymenturlcheck)

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:121](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L121)

***

### authRefreshTokenLeewaySeconds \{#authrefreshtokenleewayseconds\}

• `Optional` **authRefreshTokenLeewaySeconds**: `number`

認証を使用している場合に、トークンの有効期限が切れる何秒前にリフレッシュするかを指定します。

デフォルト値は `2` です。

#### 継承元 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[authRefreshTokenLeewaySeconds](browser.BaseConvexClientOptions.md#authrefreshtokenleewayseconds)

#### 定義場所 \{#defined-in\}

[browser/sync/client.ts:127](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L127)

***

### expectAuth \{#expectauth\}

• `Optional` **expectAuth**: `boolean`

この API は実験的です。将来的に変更または削除される可能性があります。

クエリ、ミューテーション、アクションのリクエストを、
最初の認証トークンを送信できるようになるまで保留するかどうかを指定します。

この挙動を有効にすると、認証済みクライアントのみが閲覧できるべきページで特に有用です。

デフォルトは false で、認証トークンを待ちません。

#### 継承元 \{#inherited-from\}

[BaseConvexClientOptions](browser.BaseConvexClientOptions.md).[expectAuth](browser.BaseConvexClientOptions.md#expectauth)

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:139](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L139)