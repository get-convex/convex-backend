---
id: "react"
title: "モジュール: react"
custom_edit_url: null
---

Convex を React アプリケーションに統合するためのツールです。

このモジュールには次のものが含まれます:

1. [ConvexReactClient](../classes/react.ConvexReactClient.md) — React で Convex を使用するためのクライアント。
2. [ConvexProvider](react.md#convexprovider) — このクライアントを React のコンテキストに保持するコンポーネント。
3. [Authenticated](react.md#authenticated)、[Unauthenticated](react.md#unauthenticated)、[AuthLoading](react.md#authloading) といった認証用ヘルパーコンポーネント。
4. React コンポーネントからこのクライアントにアクセスするための、[useQuery](react.md#usequery)、[useMutation](react.md#usemutation)、[useAction](react.md#useaction) などのフック。

## 使い方 \{#usage\}

### クライアントの作成 \{#creating-the-client\}

```typescript
import { ConvexReactClient } from "convex/react";

// 通常は環境変数から読み込まれる
const address = "https://small-mouse-123.convex.cloud"
const convex = new ConvexReactClient(address);
```

### React の Context にクライアントを保持する \{#storing-the-client-in-react-context\}

```typescript
import { ConvexProvider } from "convex/react";

<ConvexProvider client={convex}>
  <App />
</ConvexProvider>
```

### auth ヘルパーの使い方 \{#using-the-auth-helpers\}

```typescript
import { Authenticated, Unauthenticated, AuthLoading } from "convex/react";

<Authenticated>
  Logged in
</Authenticated>
<Unauthenticated>
  Logged out
</Unauthenticated>
<AuthLoading>
  Still loading
</AuthLoading>
```

### React Hooks を使う \{#using-react-hooks\}

```typescript
import { useQuery, useMutation } from "convex/react";
import { api } from "../convex/_generated/api";

function App() {
  const counter = useQuery(api.getCounter.default);
  const increment = useMutation(api.incrementCounter.default);
  // ここにコンポーネントを記述!
}
```

## クラス \{#classes\}

* [ConvexReactClient](../classes/react.ConvexReactClient.md)

## インターフェース \{#interfaces\}

* [ReactMutation](../interfaces/react.ReactMutation.md)
* [ReactAction](../interfaces/react.ReactAction.md)
* [Watch](../interfaces/react.Watch.md)
* [WatchQueryOptions](../interfaces/react.WatchQueryOptions.md)
* [MutationOptions](../interfaces/react.MutationOptions.md)
* [ConvexReactClientOptions](../interfaces/react.ConvexReactClientOptions.md)

## リファレンス \{#references\}

### AuthTokenFetcher \{#authtokenfetcher\}

[AuthTokenFetcher](browser.md#authtokenfetcher) を再エクスポートします

## 型エイリアス \{#type-aliases\}

### ConvexAuthState \{#convexauthstate\}

Ƭ **ConvexAuthState**: `Object`

Convex との認証連携の状態を表す型。

#### 型宣言 \{#type-declaration\}

| 名前 | 型 |
| :------ | :------ |
| `isLoading` | `boolean` |
| `isAuthenticated` | `boolean` |

#### 定義元 \{#defined-in\}

[react/ConvexAuthState.tsx:26](https://github.com/get-convex/convex-js/blob/main/src/react/ConvexAuthState.tsx#L26)

***

### OptionalRestArgsOrSkip \{#optionalrestargsorskip\}

Ƭ **OptionalRestArgsOrSkip**&lt;`FuncRef`&gt;: `FuncRef`[`"_args"`] extends `EmptyObject` ? [args?: EmptyObject | &quot;skip&quot;] : [args: FuncRef[&quot;&#95;args&quot;] | &quot;skip&quot;]

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `FuncRef` | extends [`FunctionReference`](server.md#functionreference)&lt;`any`&gt; |

#### 定義場所 \{#defined-in\}

[react/client.ts:799](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L799)

***

### Preloaded \{#preloaded\}

Ƭ **Preloaded**&lt;`Query`&gt;: `Object`

事前ロードされたクエリのペイロード。クライアントコンポーネントに渡し、
さらに [usePreloadedQuery](react.md#usepreloadedquery) に渡す必要があります。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### 型宣言 \{#type-declaration\}

| 名前 | 型 |
| :------ | :------ |
| `__type` | `Query` |
| `_name` | `string` |
| `_argsJSON` | `string` |
| `_valueJSON` | `string` |

#### 定義元 \{#defined-in\}

[react/hydration.tsx:12](https://github.com/get-convex/convex-js/blob/main/src/react/hydration.tsx#L12)

***

### PaginatedQueryReference \{#paginatedqueryreference\}

Ƭ **PaginatedQueryReference**: [`FunctionReference`](server.md#functionreference)&lt;`"query"`, `"public"`, &#123; `paginationOpts`: [`PaginationOptions`](../interfaces/server.PaginationOptions.md)  &#125;, [`PaginationResult`](../interfaces/server.PaginationResult.md)&lt;`any`&gt;&gt;

[usePaginatedQuery](react.md#usepaginatedquery) で使用できる [FunctionReference](server.md#functionreference)。

この `FunctionReference` は次の条件を満たす必要があります:

* public なクエリを参照していること
* [PaginationOptions](../interfaces/server.PaginationOptions.md) 型の `paginationOpts` という名前の引数を持つこと
* 戻り値の型が [PaginationResult](../interfaces/server.PaginationResult.md) であること

#### 定義元 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:31](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L31)

***

### UsePaginatedQueryResult \{#usepaginatedqueryresult\}

Ƭ **UsePaginatedQueryResult**&lt;`Item`&gt;: &#123; `results`: `Item`[] ; `loadMore`: (`numItems`: `number`) =&gt; `void`  &#125; &amp; &#123; `status`: `"LoadingFirstPage"` ; `isLoading`: `true`  &#125; | &#123; `status`: `"CanLoadMore"` ; `isLoading`: `false`  &#125; | &#123; `status`: `"LoadingMore"` ; `isLoading`: `true`  &#125; | &#123; `status`: `"Exhausted"` ; `isLoading`: `false`  &#125;

[usePaginatedQuery](react.md#usepaginatedquery) フックを呼び出した結果です。

この型には次のプロパティが含まれます:

* `results` - 現在読み込まれている結果の配列。
* `isLoading` - フックが現在結果を読み込み中かどうか。
* `status` - ページネーションの状態。取りうる状態は次のとおりです:
  * &quot;LoadingFirstPage&quot;: 最初のページの結果を読み込んでいる状態。
  * &quot;CanLoadMore&quot;: まだ取得可能なアイテムがある可能性がある状態。別のページを取得するには `loadMore` を呼び出します。
  * &quot;LoadingMore&quot;: 別のページの結果を読み込み中の状態。
  * &quot;Exhausted&quot;: リストの末尾までページネーションが完了した状態。
* `loadMore(n)` - さらに結果を取得するためのコールバック。`status` が &quot;CanLoadMore&quot; の場合にのみ、追加の結果を取得します。

#### 型パラメータ \{#type-parameters\}

| 名前 |
| :------ |
| `Item` |

#### 定義場所 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:479](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L479)

***

### PaginationStatus \{#paginationstatus\}

Ƭ **PaginationStatus**: [`UsePaginatedQueryResult`](react.md#usepaginatedqueryresult)&lt;`any`&gt;[`"status"`]

[`UsePaginatedQueryResult`](react.md#usepaginatedqueryresult) における、取りうるページネーションの status 値です。

これは文字列リテラル型のユニオン型です。

#### 定義元 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:507](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L507)

***

### PaginatedQueryArgs \{#paginatedqueryargs\}

Ƭ **PaginatedQueryArgs**&lt;`Query`&gt;: [`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt;

[PaginatedQueryReference](react.md#paginatedqueryreference) が与えられたときに、`paginationOpts` 引数を除いたクエリの引数オブジェクト型を取得します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### 定義場所 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:515](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L515)

***

### PaginatedQueryItem \{#paginatedqueryitem\}

Ƭ **PaginatedQueryItem**&lt;`Query`&gt;: [`FunctionReturnType`](server.md#functionreturntype)&lt;`Query`&gt;[`"page"`][`number`]

[PaginatedQueryReference](react.md#paginatedqueryreference) が与えられたときに、ページネーション対象となるアイテムの型を取得します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### 定義元 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:524](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L524)

***

### UsePaginatedQueryReturnType \{#usepaginatedqueryreturntype\}

Ƭ **UsePaginatedQueryReturnType**&lt;`Query`&gt;: [`UsePaginatedQueryResult`](react.md#usepaginatedqueryresult)&lt;[`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt;&gt;

[usePaginatedQuery](react.md#usepaginatedquery) の戻り値の型です。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### 定義元 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:532](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L532)

***

### RequestForQueries \{#requestforqueries\}

Ƭ **RequestForQueries**: `Record`&lt;`string`, &#123; `query`: [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; ; `args`: `Record`&lt;`string`, [`Value`](values.md#value)&gt;  &#125;&gt;

複数のクエリをロードするためのリクエストを表すオブジェクト。

このオブジェクトのキーは識別子であり、値はクエリ関数とその関数に渡す引数を含むオブジェクトです。

これは [useQueries](react.md#usequeries) フックへの引数として使用されます。

#### 定義場所 \{#defined-in\}

[react/use&#95;queries.ts:137](https://github.com/get-convex/convex-js/blob/main/src/react/use_queries.ts#L137)

## 関数 \{#functions\}

### useConvexAuth \{#useconvexauth\}

▸ **useConvexAuth**(): `Object`

React コンポーネント内で [ConvexAuthState](react.md#convexauthstate) を取得します。

これは、React コンポーネントツリーの上位に Convex の認証インテグレーション用プロバイダーが存在することに依存します。

#### 戻り値 \{#returns\}

`Object`

現在の [ConvexAuthState](react.md#convexauthstate)。

| 名前 | 型 |
| :------ | :------ |
| `isLoading` | `boolean` |
| `isAuthenticated` | `boolean` |

#### 定義場所 \{#defined-in\}

[react/ConvexAuthState.tsx:43](https://github.com/get-convex/convex-js/blob/main/src/react/ConvexAuthState.tsx#L43)

***

### ConvexProviderWithAuth \{#convexproviderwithauth\}

▸ **ConvexProviderWithAuth**(`«destructured»`): `Element`

[ConvexProvider](react.md#convexprovider) の代わりとなるコンポーネントであり、このコンポーネントの子孫に
[ConvexAuthState](react.md#convexauthstate) も提供します。

任意の認証プロバイダーを Convex と統合するために使用します。`useAuth` prop は、
プロバイダーの認証状態と、JWT アクセストークンを取得する関数を返す React フックである必要があります。

`useAuth` prop 関数の更新によって再レンダーが発生した場合、認証状態は `loading` に遷移し、
`fetchAccessToken()` 関数が再度呼び出されます。

詳しくは [Custom Auth Integration](https://docs.convex.dev/auth/advanced/custom-auth) を参照してください。

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children?` | `ReactNode` |
| › `client` | `IConvexReactClient` |
| › `useAuth` | () =&gt; &#123; `isLoading`: `boolean` ; `isAuthenticated`: `boolean` ; `fetchAccessToken`: (`args`: &#123; `forceRefreshToken`: `boolean`  &#125;) =&gt; `Promise`&lt;`null` | `string`&gt;  &#125; |

#### 戻り値 \{#returns\}

`Element`

#### 定義場所 \{#defined-in\}

[react/ConvexAuthState.tsx:75](https://github.com/get-convex/convex-js/blob/main/src/react/ConvexAuthState.tsx#L75)

***

### Authenticated \{#authenticated\}

▸ **Authenticated**(`«destructured»`): `null` | `Element`

クライアントが認証済みであれば、子要素をレンダーします。

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children` | `ReactNode` |

#### 戻り値 \{#returns\}

`null` | `Element`

#### 定義元 \{#defined-in\}

[react/auth&#95;helpers.tsx:10](https://github.com/get-convex/convex-js/blob/main/src/react/auth_helpers.tsx#L10)

***

### Unauthenticated \{#unauthenticated\}

▸ **Unauthenticated**(`«destructured»`): `null` | `Element`

クライアントが認証を利用しているが認証済みではない場合に、子要素をレンダーします。

#### パラメータ \{#parameters\}

| 名称 | 型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children` | `ReactNode` |

#### 戻り値 \{#returns\}

`null` | `Element`

#### 定義元 \{#defined-in\}

[react/auth&#95;helpers.tsx:23](https://github.com/get-convex/convex-js/blob/main/src/react/auth_helpers.tsx#L23)

***

### AuthLoading \{#authloading\}

▸ **AuthLoading**(`«destructured»`): `null` | `Element`

クライアントが認証を利用していないか、認証処理の最中である場合に子要素をレンダーします。

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children` | `ReactNode` |

#### 戻り値 \{#returns\}

`null` | `Element`

#### 定義場所 \{#defined-in\}

[react/auth&#95;helpers.tsx:37](https://github.com/get-convex/convex-js/blob/main/src/react/auth_helpers.tsx#L37)

***

### useConvex \{#useconvex\}

▸ **useConvex**(): [`ConvexReactClient`](../classes/react.ConvexReactClient.md)

React コンポーネント内で [ConvexReactClient](../classes/react.ConvexReactClient.md) を取得します。

これは、React コンポーネントツリーの上位に [ConvexProvider](react.md#convexprovider) が配置されていることに依存します。

#### 戻り値 \{#returns\}

[`ConvexReactClient`](../classes/react.ConvexReactClient.md)

現在の [ConvexReactClient](../classes/react.ConvexReactClient.md) オブジェクト、または `undefined`。

#### 定義場所 \{#defined-in\}

[react/client.ts:774](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L774)

***

### ConvexProvider \{#convexprovider\}

▸ **ConvexProvider**(`props`, `deprecatedLegacyContext?`): `null` | `ReactElement`&lt;`any`, `any`&gt;

このコンポーネント配下のコンポーネントに対して、アクティブな Convex の [ConvexReactClient](../classes/react.ConvexReactClient.md) を提供します。

Convex のフック `useQuery`、`useMutation`、`useConvex` を使うには、アプリ全体をこのコンポーネントでラップしてください。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `props` | `Object` | [ConvexReactClient](../classes/react.ConvexReactClient.md) を指す `client` プロパティを持つオブジェクト。 |
| `props.client` | [`ConvexReactClient`](../classes/react.ConvexReactClient.md) | - |
| `props.children?` | `ReactNode` | - |
| `deprecatedLegacyContext?` | `any` | **`非推奨`** **`詳細`** は [React Docs](https://legacy.reactjs.org/docs/legacy-context.html#referencing-context-in-lifecycle-methods) を参照してください |

#### 戻り値 \{#returns\}

`null` | `ReactElement`&lt;`any`, `any`&gt;

#### 定義場所 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/@types+react@18.3.26/node&#95;modules/@types/react/ts5.0/index.d.ts:1129

***

### useQuery \{#usequery\}

▸ **useQuery**&lt;`Query`&gt;(`query`, `...args`): `Query`[`"_returnType"`] | `undefined`

React コンポーネント内でリアクティブなクエリを読み込みます。

この React フックは内部状態を保持しており、クエリ結果が変化するたびに
再レンダーを引き起こします。

[ConvexProvider](react.md#convexprovider) の配下で使用されていない場合はエラーをスローします。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | 実行するパブリックなクエリを指す [FunctionReference](server.md#functionreference) で、`api.dir1.dir2.filename.func` のような形式です。 |
| `...args` | [`OptionalRestArgsOrSkip`](react.md#optionalrestargsorskip)&lt;`Query`&gt; | クエリ関数に渡す引数、またはクエリをロードしない場合に指定する文字列 `skip`。 |

#### 戻り値 \{#returns\}

`Query`[`"_returnType"`] | `undefined`

クエリの結果を返します。クエリがロード中の場合は `undefined` を返します。

#### 定義元 \{#defined-in\}

[react/client.ts:820](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L820)

***

### useMutation \{#usemutation\}

▸ **useMutation**&lt;`Mutation`&gt;(`mutation`): [`ReactMutation`](../interfaces/react.ReactMutation.md)&lt;`Mutation`&gt;

新しい [ReactMutation](../interfaces/react.ReactMutation.md) を生成します。

`Mutation` オブジェクトは関数のように呼び出して、対応する Convex 関数の実行を要求したり、
[楽観的更新](https://docs.convex.dev/using/optimistic-updates) を使ってさらに構成したりできます。

このフックから返される値はレンダリング間で安定しているため、オブジェクト同一性に依存する
React の依存配列やメモ化ロジックで使用しても、再レンダリングを引き起こしません。

[ConvexProvider](react.md#convexprovider) の配下以外で使用した場合はエラーをスローします。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](server.md#functionreference)&lt;`"mutation"`&gt; |

#### パラメーター \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | `api.dir1.dir2.filename.func` のように実行する公開ミューテーションを指定するための [FunctionReference](server.md#functionreference)。 |

#### 戻り値 \{#returns\}

[`ReactMutation`](../interfaces/react.ReactMutation.md)&lt;`Mutation`&gt;

その名前の [ReactMutation](../interfaces/react.ReactMutation.md) オブジェクト。

#### 定義元 \{#defined-in\}

[react/client.ts:872](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L872)

***

### useAction \{#useaction\}

▸ **useAction**&lt;`Action`&gt;(`action`): [`ReactAction`](../interfaces/react.ReactAction.md)&lt;`Action`&gt;

新しい [ReactAction](../interfaces/react.ReactAction.md) を作成します。

`Action` オブジェクトは関数のように呼び出すことができ、対応する Convex 関数の
実行をリクエストできます。

このフックが返す値はレンダリング間で安定しているため、React の依存配列やオブジェクト
同一性に依存するメモ化ロジックで使用しても、コンポーネントの再レンダリングを引き起こしません。

[ConvexProvider](react.md#convexprovider) の配下で使用されていない場合はエラーをスローします。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](server.md#functionreference)&lt;`"action"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `action` | `Action` | 実行する公開アクションを表す [FunctionReference](server.md#functionreference)。`api.dir1.dir2.filename.func` のような形式で指定します。 |

#### 戻り値 \{#returns\}

[`ReactAction`](../interfaces/react.ReactAction.md)&lt;`Action`&gt;

その名前の [ReactAction](../interfaces/react.ReactAction.md) オブジェクト。

#### 定義元 \{#defined-in\}

[react/client.ts:913](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L913)

***

### useConvexConnectionState \{#useconvexconnectionstate\}

▸ **useConvexConnectionState**(): [`ConnectionState`](browser.md#connectionstate)

現在の [ConnectionState](browser.md#connectionstate) を取得し、その変化を購読するための React フックです。

このフックは現在の接続状態を返し、接続状態のいずれかの部分が変化したとき
（オンライン/オフラインの切り替え、リクエストの開始/完了など）に自動的に再レンダーされます。

ConnectionState の構造は将来変更される可能性があり、その結果として
このフックがより頻繁に再レンダーされる場合があります。

[ConvexProvider](react.md#convexprovider) の配下以外で使用された場合はエラーをスローします。

#### 戻り値 \{#returns\}

[`ConnectionState`](browser.md#connectionstate)

Convex バックエンドとの現在の接続状態を示す [`ConnectionState`](browser.md#connectionstate)。

#### 定義場所 \{#defined-in\}

[react/client.ts:952](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L952)

***

### usePreloadedQuery \{#usepreloadedquery\}

▸ **usePreloadedQuery**&lt;`Query`&gt;(`preloadedQuery`): `Query`[`"_returnType"`]

Server Component から返される [preloadQuery](nextjs.md#preloadquery) の `Preloaded` ペイロードを使って、
React コンポーネント内でリアクティブなクエリを読み込みます。

この React フックは内部状態を持っており、クエリ結果が変化するたびに
再レンダリングを引き起こします。

[ConvexProvider](react.md#convexprovider) の配下で使用されていない場合はエラーをスローします。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](server.md#functionreference)&lt;`"query"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `preloadedQuery` | [`Preloaded`](react.md#preloaded)&lt;`Query`&gt; | Server Component から渡される `Preloaded` クエリのペイロード。 |

#### 返り値 \{#returns\}

`Query`[`"_returnType"`]

クエリの結果。最初は Server Component で取得された結果を返し、その後はクライアントで取得された結果を返します。

#### 定義場所 \{#defined-in\}

[react/hydration.tsx:34](https://github.com/get-convex/convex-js/blob/main/src/react/hydration.tsx#L34)

***

### usePaginatedQuery \{#usepaginatedquery\}

▸ **usePaginatedQuery**&lt;`Query`&gt;(`query`, `args`, `options`): [`UsePaginatedQueryReturnType`](react.md#usepaginatedqueryreturntype)&lt;`Query`&gt;

ページネーション対応のクエリからデータをリアクティブに読み込み、順次長くなるリストを作成します。

これは「インフィニットスクロール」UIを実装するために使用できます。

このフックは、[PaginatedQueryReference](react.md#paginatedqueryreference)
に一致する公開クエリ参照と一緒に使用する必要があります。

`usePaginatedQuery` は、すべての結果ページを 1 つのリストに連結し、
さらに項目をリクエストする際の継続用カーソルを管理します。

使用例:

```typescript
const { results, status, isLoading, loadMore } = usePaginatedQuery(
  api.messages.list,
  { channel: "#general" },
  { initialNumItems: 5 }
);
```

クエリの参照または引数が変更されると、ページネーションの状態は最初のページにリセットされます。同様に、いずれかのページで InvalidCursor エラーやデータ量が多すぎることに関連するエラーが発生した場合も、ページネーションの状態は最初のページにリセットされます。

ページネーションの詳細については、[Paginated Queries](https://docs.convex.dev/database/pagination) を参照してください。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | 実行する公開クエリ関数への `FunctionReference`。 |
| `args` | `"skip"` | [`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt; | クエリ関数に渡す引数オブジェクト（`paginationOpts` プロパティを除いたもの）。このプロパティはこのフックによって注入されます。 |
| `options` | `Object` | 最初のページで読み込む `initialNumItems` を指定するオブジェクト。 |
| `options.initialNumItems` | `number` | - |

#### 戻り値 \{#returns\}

[`UsePaginatedQueryReturnType`](react.md#usepaginatedqueryreturntype)&lt;`Query`&gt;

現在読み込まれているアイテム、ページネーションの status、そして `loadMore` 関数を含む [UsePaginatedQueryResult](react.md#usepaginatedqueryresult) を返します。

#### 定義元 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:162](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L162)

***

### resetPaginationId \{#resetpaginationid\}

▸ **resetPaginationId**(): `void`

テスト専用にページネーション ID をリセットし、テストでその値を把握できるようにします。

#### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:458](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L458)

***

### optimisticallyUpdateValueInPaginatedQuery \{#optimisticallyupdatevalueinpaginatedquery\}

▸ **optimisticallyUpdateValueInPaginatedQuery**&lt;`Query`&gt;(`localStore`, `query`, `args`, `updateValue`): `void`

ページネーションされたリスト内の値を楽観的に更新します。

この楽観的更新は、
[usePaginatedQuery](react.md#usepaginatedquery) で読み込まれたデータを更新するために使用されることを想定しています。読み込まれているすべてのページにわたってリスト内の各要素に `updateValue` を適用し、リストを更新します。

これは、名前と引数が一致するクエリにのみ適用されます。

使用例：

```ts
const myMutation = useMutation(api.myModule.myMutation)
.withOptimisticUpdate((localStore, mutationArg) => {

  // ID `mutationArg` を持つドキュメントに追加のプロパティを持たせるよう
  // 楽観的に更新します。

  optimisticallyUpdateValueInPaginatedQuery(
    localStore,
    api.myModule.paginatedQuery
    {},
    currentValue => {
      if (mutationArg === currentValue._id) {
        return {
          ...currentValue,
          "newProperty": "newValue",
        };
      }
      return currentValue;
    }
  );

});
```

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `localStore` | [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md) | 更新対象の [OptimisticLocalStore](../interfaces/browser.OptimisticLocalStore.md)。 |
| `query` | `Query` | 更新対象のページネーションされたクエリの [FunctionReference](server.md#functionreference)。 |
| `args` | [`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt; | `paginationOpts` プロパティを除いた、クエリ関数に渡す引数オブジェクト。 |
| `updateValue` | (`currentValue`: [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt;) =&gt; [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt; | 新しい値を生成する関数。 |

#### 戻り値 \{#returns\}

`void`

#### 定義元 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:578](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L578)

***

### insertAtTop \{#insertattop\}

▸ **insertAtTop**&lt;`Query`&gt;(`options`): `void`

ページネーションされたクエリを更新し、要素をリストの先頭に挿入します。

これはソート順に関係なく行われます。そのため、リストが降順の場合は
挿入された要素は「最大」の要素として扱われ、昇順の場合は
「最小」の要素として扱われます。

例:

```ts
const createTask = useMutation(api.tasks.create)
  .withOptimisticUpdate((localStore, mutationArgs) => {
  insertAtTop({
    paginatedQuery: api.tasks.list,
    argsToMatch: { listId: mutationArgs.listId },
    localQueryStore: localStore,
    item: { _id: crypto.randomUUID() as Id<"tasks">, title: mutationArgs.title, completed: false },
  });
});
```

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### パラメーター \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `options` | `Object` | - |
| `options.paginatedQuery` | `Query` | ページネーションされたクエリ関数への参照。 |
| `options.argsToMatch?` | `Partial`&lt;[`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt;&gt; | 対象となる各ページネーションされたクエリに共通して含まれている必要がある任意の引数。 同じ Query 関数を異なる引数で呼び出して別々のリストを読み込む場合に便利です。 |
| `options.localQueryStore` | [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md) |  |
| `options.item` | [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt; | 挿入するアイテム。 |

#### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:640](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L640)

***

### insertAtBottomIfLoaded \{#insertatbottomifloaded\}

▸ **insertAtBottomIfLoaded**&lt;`Query`&gt;(`options`): `void`

ページネーションされたクエリを、リストの一番下に要素を挿入するように更新します。

ソート順に関係なく動作するため、リストが降順の場合は挿入された要素は「最小」の要素として扱われ、昇順の場合は「最大」の要素として扱われます。

この関数が効果を持つのは、最後のページが読み込まれている場合のみです。そうでない場合、読み込まれている部分（リストの途中）の末尾に要素が挿入され、その後楽観的更新が終了するとその要素はリストから外れてしまいます。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `options` | `Object` | - |
| `options.paginatedQuery` | `Query` | ページネーションされたクエリへの関数への参照。 |
| `options.argsToMatch?` | `Partial`&lt;[`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt;&gt; | 関連する各ページネーション済みクエリに共通して含まれている必要がある省略可能な引数。同じクエリ関数を、異なる引数で呼び出して異なるリストをロードする場合に便利です。 |
| `options.localQueryStore` | [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md) |  |
| `options.item` | [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt; | - |

#### 戻り値 \{#returns\}

`void`

#### 定義元 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:689](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L689)

***

### insertAtPosition \{#insertatposition\}

▸ **insertAtPosition**&lt;`Query`&gt;(`options`): `void`

これは、ページネーションされたクエリ内の特定の位置にアイテムを挿入するためのヘルパー関数です。

`sortOrder` と、リスト内のアイテムからソートキー（値の配列）を算出する関数を指定する必要があります。

これは、サーバー側のクエリが楽観的更新と同じソート順とソートキーを使用している場合にのみ機能します。

例:

```ts
const createTask = useMutation(api.tasks.create)
  .withOptimisticUpdate((localStore, mutationArgs) => {
  insertAtPosition({
    paginatedQuery: api.tasks.listByPriority,
    argsToMatch: { listId: mutationArgs.listId },
    sortOrder: "asc",
    sortKeyFromItem: (item) => [item.priority, item._creationTime],
    localQueryStore: localStore,
    item: {
      _id: crypto.randomUUID() as Id<"tasks">,
      _creationTime: Date.now(),
      title: mutationArgs.title,
      completed: false,
      priority: mutationArgs.priority,
    },
  });
});
```

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `options` | `Object` | - |
| `options.paginatedQuery` | `Query` | ページネートされたクエリへの関数参照。 |
| `options.argsToMatch?` | `Partial`&lt;[`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;[`FunctionArgs`](server.md#functionargs)&lt;`Query`&gt;, `"paginationOpts"`&gt;&gt;&gt; | 関連する各ページネートされたクエリに共通して含まれている必要があるオプション引数。同じクエリ関数を異なる引数で呼び出して別々のリストを読み込む場合に便利です。 |
| `options.sortOrder` | `"asc"` | `"desc"` | ページネートされたクエリのソート順（&quot;asc&quot; または &quot;desc&quot;）。 |
| `options.sortKeyFromItem` | (`element`: [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt;) =&gt; [`Value`](values.md#value) | [`Value`](values.md#value)[] | リスト内の要素からソートキー（値の配列）を導出するための関数です。`_creationTime` のようなタイブレーク用フィールドを含めることを推奨します。 |
| `options.localQueryStore` | [`OptimisticLocalStore`](../interfaces/browser.OptimisticLocalStore.md) |  |
| `options.item` | [`PaginatedQueryItem`](react.md#paginatedqueryitem)&lt;`Query`&gt; | 挿入するアイテム。 |

#### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[react/use&#95;paginated&#95;query.ts:770](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query.ts#L770)

***

### usePaginatedQuery_experimental \{#usepaginatedquery_experimental\}

▸ **usePaginatedQuery&#95;experimental**&lt;`Query`&gt;(`query`, `args`, `options`): [`UsePaginatedQueryReturnType`](react.md#usepaginatedqueryreturntype)&lt;`Query`&gt;

将来的に現在の実装を置き換える予定の、新しい usePaginatedQuery の実験的な実装です。

ページネーションされたクエリからデータをリアクティブに読み込み、増えていくリストを作成します。

これは、新しいクライアント側のページネーションロジックに依存した代替実装です。

これを使って「インフィニットスクロール」UIを実現できます。

このフックは、[PaginatedQueryReference](react.md#paginatedqueryreference) に一致する public なクエリ参照と一緒に使用する必要があります。

`usePaginatedQuery` は、すべての結果ページを 1 つのリストに連結し、
さらにアイテムを要求する際の継続カーソルを管理します。

使用例:

```typescript
const { results, status, isLoading, loadMore } = usePaginatedQuery(
  api.messages.list,
  { channel: "#general" },
  { initialNumItems: 5 }
);
```

クエリの参照または引数が変更された場合、ページネーションの状態は最初のページにリセットされます。同様に、いずれかのページで InvalidCursor エラーや、データ量が多すぎることに関連するエラーが発生した場合も、ページネーションの状態は最初のページにリセットされます。

ページネーションの詳細については、[Paginated Queries](https://docs.convex.dev/database/pagination) を参照してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`PaginatedQueryReference`](react.md#paginatedqueryreference) |

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | 実行するパブリックなクエリ関数を指す FunctionReference。 |
| `args` | `"skip"` | [`PaginatedQueryArgs`](react.md#paginatedqueryargs)&lt;`Query`&gt; | クエリ関数に渡す引数オブジェクト。ただし `paginationOpts` プロパティは除きます。このプロパティはこのフックによって自動的に設定されます。 |
| `options` | `Object` | 最初のページで読み込む `initialNumItems` を指定するオブジェクト。 |
| `options.initialNumItems` | `number` | - |

#### 戻り値 \{#returns\}

[`UsePaginatedQueryReturnType`](react.md#usepaginatedqueryreturntype)&lt;`Query`&gt;

現在読み込まれているアイテム、ページネーションの状態、および `loadMore` 関数を含む [UsePaginatedQueryResult](react.md#usepaginatedqueryresult)。

#### 定義元 \{#defined-in\}

[react/use&#95;paginated&#95;query2.ts:72](https://github.com/get-convex/convex-js/blob/main/src/react/use_paginated_query2.ts#L72)

***

### useQueries \{#usequeries\}

▸ **useQueries**(`queries`): `Record`&lt;`string`, `any` | `undefined` | `Error`&gt;

可変個数のリアクティブな Convex クエリをロードします。

`useQueries` は [useQuery](react.md#usequery) と似ていますが、
複数のクエリをロードできる点が異なります。これにより、React フックの
ルールに違反することなく、動的な数のクエリを扱うのに役立ちます。

このフックは、各クエリの識別子をキーとし、
値を `{ query: FunctionReference, args: Record<string, Value> }` というオブジェクトにした
オブジェクトを受け取ります。`query` はロードする Convex クエリ関数の
FunctionReference であり、`args` はその関数への引数です。

このフックは各識別子をクエリの結果にマッピングしたオブジェクトを返します。
クエリがまだロード中の場合は `undefined`、クエリが例外をスローした場合は
`Error` のインスタンスを返します。

たとえば次のようにクエリをロードした場合:

```typescript
const results = useQueries({
  messagesInGeneral: {
    query: "listMessages",
    args: { channel: "#general" }
  }
});
```

結果は次のようになります:

```typescript
{
  messagesInGeneral: [{
    channel: "#general",
    body: "hello"
    _id: ...,
    _creationTime: ...
  }]
}
```

この React フックは内部状態を持っており、いずれかのクエリ結果が変更されるたびに再レンダーが発生します。

[ConvexProvider](react.md#convexprovider) の配下で使用されていない場合はエラーをスローします。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `queries` | [`RequestForQueries`](react.md#requestforqueries) | どのクエリ関数を取得するかを指定するために、識別子を `{query: string, args: Record&lt;string, Value&gt; }` オブジェクトに対応付けるオブジェクト。 |

#### 戻り値 \{#returns\}

`Record`&lt;`string`, `any` | `undefined` | `Error`&gt;

入力と同じキーを持つオブジェクトです。値はクエリ関数の結果であり、読み込み中であれば `undefined`、例外がスローされた場合は `Error` になります。

#### 定義元 \{#defined-in\}

[react/use&#95;queries.ts:61](https://github.com/get-convex/convex-js/blob/main/src/react/use_queries.ts#L61)