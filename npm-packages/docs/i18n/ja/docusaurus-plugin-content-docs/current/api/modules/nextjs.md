---
id: "nextjs"
title: "モジュール: nextjs"
custom_edit_url: null
---

サーバーレンダリングを利用する Next.js アプリケーションに Convex を統合するためのヘルパー関数群です。

このモジュールには次が含まれます:

1. リアクティブなクライアントコンポーネント向けのデータをプリロードするための [preloadQuery](nextjs.md#preloadquery)。
2. Next.js の Server Components、Server Actions、Route Handlers から Convex データを読み込み・更新するための
   [fetchQuery](nextjs.md#fetchquery)、[fetchMutation](nextjs.md#fetchmutation)、[fetchAction](nextjs.md#fetchaction)。

## 使用方法 \{#usage\}

エクスポートされるすべての関数は、`NEXT_PUBLIC_CONVEX_URL` 環境変数に Convex のデプロイメントURL が設定されていることを前提として動作します。ローカル開発時には、`npx convex dev` がこれを自動的に設定します。

### データの事前読み込み \{#preloading-data\}

Server Component 内でデータを事前読み込みします:

```typescript
import { preloadQuery } from "convex/nextjs";
import { api } from "@/convex/_generated/api";
import ClientComponent from "./ClientComponent";

export async function ServerComponent() {
  const preloaded = await preloadQuery(api.foo.baz);
  return <ClientComponent preloaded={preloaded} />;
}
```

そして、これを Client コンポーネントに渡します:

```typescript
import { Preloaded, usePreloadedQuery } from "convex/react";
import { api } from "@/convex/_generated/api";

export function ClientComponent(props: {
  preloaded: Preloaded<typeof api.foo.baz>;
}) {
  const data = usePreloadedQuery(props.preloaded);
  // `data`をレンダリング...
}
```

## 型エイリアス \{#type-aliases\}

### NextjsOptions \{#nextjsoptions\}

Ƭ **NextjsOptions**: `Object`

[preloadQuery](nextjs.md#preloadquery)、[fetchQuery](nextjs.md#fetchquery)、[fetchMutation](nextjs.md#fetchmutation)、[fetchAction](nextjs.md#fetchaction) に渡すオプション。

#### 型宣言 \{#type-declaration\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `token?` | `string` | 関数呼び出しに使用する、JWT 形式でエンコードされた OpenID Connect 認証トークン。 |
| `url?` | `string` | 関数呼び出しに使用する Convex デプロイメントURL。指定されていない場合は `process.env.NEXT_PUBLIC_CONVEX_URL` が既定値として使用されます。ここに（環境変数の未設定などにより）明示的に `undefined` を渡すと、今後エラーがスローされるようになります。 |
| `skipConvexDeploymentUrlCheck?` | `boolean` | Convex デプロイメントURLが `https://happy-animal-123.convex.cloud` または localhost のような形式であることの検証をスキップします。これは、別の URL を使用するセルフホスト型の Convex バックエンドを実行している場合に有用です。デフォルト値は `false` です。 |

#### 定義元 \{#defined-in\}

[nextjs/index.ts:60](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L60)

## 関数 \{#functions\}

### preloadQuery \{#preloadquery\}

▸ **preloadQuery**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`Preloaded`](react.md#preloaded)&lt;`Query`&gt;&gt;

Convex のクエリ関数を実行して、Client Component で [usePreloadedQuery](react.md#usepreloadedquery) に渡せる `Preloaded` ペイロードを返します。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | 実行する公開クエリを指定する [FunctionReference](server.md#functionreference)。例: `api.dir1.dir2.filename.func`。 |
| `...args` | [`ArgsAndOptions`](server.md#argsandoptions)&lt;`Query`, [`NextjsOptions`](nextjs.md#nextjsoptions)&gt; | クエリの引数オブジェクト。これを省略すると、引数は `{}` になります。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`Preloaded`](react.md#preloaded)&lt;`Query`&gt;&gt;

`Preloaded` ペイロードを返す Promise。

#### 定義場所 \{#defined-in\}

[nextjs/index.ts:101](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L101)

***

### preloadedQueryResult \{#preloadedqueryresult\}

▸ **preloadedQueryResult**&lt;`Query`&gt;(`preloaded`): [`FunctionReturnType`](server.md#functionreturntype)&lt;`Query`&gt;

[preloadQuery](nextjs.md#preloadquery) を使って実行されたクエリの結果を返します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `preloaded` | [`Preloaded`](react.md#preloaded)&lt;`Query`&gt; | [preloadQuery](nextjs.md#preloadquery) によって返される `Preloaded` のペイロード。 |

#### 戻り値 \{#returns\}

[`FunctionReturnType`](server.md#functionreturntype)&lt;`Query`&gt;

クエリの結果。

#### 定義元 \{#defined-in\}

[nextjs/index.ts:120](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L120)

***

### fetchQuery \{#fetchquery\}

▸ **fetchQuery**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Query`&gt;&gt;

Convex のクエリ関数を実行します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | 実行する公開クエリを指定する [FunctionReference](server.md#functionreference)。`api.dir1.dir2.filename.func` のように指定します。 |
| `...args` | [`ArgsAndOptions`](server.md#argsandoptions)&lt;`Query`, [`NextjsOptions`](nextjs.md#nextjsoptions)&gt; | クエリ用の引数オブジェクトです。省略した場合、引数は `{}` になります。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Query`&gt;&gt;

クエリの結果で解決される `Promise`。

#### 定義場所 \{#defined-in\}

[nextjs/index.ts:136](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L136)

***

### fetchMutation \{#fetchmutation\}

▸ **fetchMutation**&lt;`Mutation`&gt;(`mutation`, `...args`): `Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

Convexのミューテーション関数を実行します。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](server.md#functionreference)&lt;`"mutation"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | `api.dir1.dir2.filename.func` のような形式で実行するパブリックなミューテーションを指す [FunctionReference](server.md#functionreference)。 |
| `...args` | [`ArgsAndOptions`](server.md#argsandoptions)&lt;`Mutation`, [`NextjsOptions`](nextjs.md#nextjsoptions)&gt; | ミューテーションに渡す引数オブジェクト。省略された場合、引数オブジェクトは `{}` になります。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

ミューテーションの結果を返す Promise。

#### 定義元 \{#defined-in\}

[nextjs/index.ts:155](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L155)

***

### fetchAction \{#fetchaction\}

▸ **fetchAction**&lt;`Action`&gt;(`action`, `...args`): `Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Action`&gt;&gt;

Convex のアクション関数を実行します。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](server.md#functionreference)&lt;`"action"`&gt; |

#### パラメーター \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `action` | `Action` | `api.dir1.dir2.filename.func` のように実行する公開アクションのための [FunctionReference](server.md#functionreference)。 |
| `...args` | [`ArgsAndOptions`](server.md#argsandoptions)&lt;`Action`, [`NextjsOptions`](nextjs.md#nextjsoptions)&gt; | アクションに渡す引数オブジェクト。省略すると、引数は `{}` になります。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](server.md#functionreturntype)&lt;`Action`&gt;&gt;

アクションの結果を解決する Promise。

#### 定義場所 \{#defined-in\}

[nextjs/index.ts:176](https://github.com/get-convex/convex-js/blob/main/src/nextjs/index.ts#L176)