---
id: "server"
title: "モジュール: server"
custom_edit_url: null
---

サーバー側の Convex クエリおよびミューテーション関数を実装するためのユーティリティです。

## 使い方 \{#usage\}

### コード生成 \{#code-generation\}

このモジュールは通常、自動生成されたサーバーコードと一緒に使用します。

サーバーコードを生成するには、Convex プロジェクト内で `npx convex dev` を実行します。
これにより、スキーマに基づいて型情報が付与された次の関数を含む
`convex/_generated/server.js` ファイルが作成されます:

* [query](https://docs.convex.dev/generated-api/server#query)
* [mutation](https://docs.convex.dev/generated-api/server#mutation)

TypeScript とコード生成を使用していない場合は、代わりに次の型付けされていない
関数を使用できます:

* [queryGeneric](server.md#querygeneric)
* [mutationGeneric](server.md#mutationgeneric)

### 例 \{#example\}

Convex 関数は `query` または
`mutation` のラッパーを使って定義します。

クエリは、[GenericDatabaseReader](../interfaces/server.GenericDatabaseReader.md) インターフェースを実装した `db` を受け取ります。

```js
import { query } from "./_generated/server";

export default query({
  handler: async ({ db }, { arg1, arg2 }) => {
    // ここに(読み取り専用の)コードを記述してください!
  },
});
```

関数がドキュメントの挿入、更新、削除などデータベースへの書き込みを行う必要がある場合は、代わりに `mutation` を使用してください。`mutation` では、[GenericDatabaseWriter](../interfaces/server.GenericDatabaseWriter.md) インターフェースを実装する `db` が提供されます。

```js
import { mutation } from "./_generated/server";

export default mutation({
  handler: async ({ db }, { arg1, arg2 }) => {
    // ミューテーションのコードをここに書いてください!
  },
});
```

## クラス \{#classes\}

* [Crons](../classes/server.Crons.md)
* [Expression](../classes/server.Expression.md)
* [IndexRange](../classes/server.IndexRange.md)
* [HttpRouter](../classes/server.HttpRouter.md)
* [TableDefinition](../classes/server.TableDefinition.md)
* [SchemaDefinition](../classes/server.SchemaDefinition.md)
* [SearchFilter](../classes/server.SearchFilter.md)
* [FilterExpression](../classes/server.FilterExpression.md)

## インターフェース \{#interfaces\}

* [UserIdentity](../interfaces/server.UserIdentity.md)
* [Auth](../interfaces/server.Auth.md)
* [CronJob](../interfaces/server.CronJob.md)
* [BaseTableReader](../interfaces/server.BaseTableReader.md)
* [GenericDatabaseReader](../interfaces/server.GenericDatabaseReader.md)
* [GenericDatabaseReaderWithTable](../interfaces/server.GenericDatabaseReaderWithTable.md)
* [GenericDatabaseWriter](../interfaces/server.GenericDatabaseWriter.md)
* [GenericDatabaseWriterWithTable](../interfaces/server.GenericDatabaseWriterWithTable.md)
* [BaseTableWriter](../interfaces/server.BaseTableWriter.md)
* [FilterBuilder](../interfaces/server.FilterBuilder.md)
* [IndexRangeBuilder](../interfaces/server.IndexRangeBuilder.md)
* [PaginationResult](../interfaces/server.PaginationResult.md)
* [PaginationOptions](../interfaces/server.PaginationOptions.md)
* [QueryInitializer](../interfaces/server.QueryInitializer.md)
* [Query](../interfaces/server.Query.md)
* [OrderedQuery](../interfaces/server.OrderedQuery.md)
* [GenericMutationCtx](../interfaces/server.GenericMutationCtx.md)
* [GenericQueryCtx](../interfaces/server.GenericQueryCtx.md)
* [GenericActionCtx](../interfaces/server.GenericActionCtx.md)
* [ValidatedFunction](../interfaces/server.ValidatedFunction.md)
* [Scheduler](../interfaces/server.Scheduler.md)
* [SearchIndexConfig](../interfaces/server.SearchIndexConfig.md)
* [VectorIndexConfig](../interfaces/server.VectorIndexConfig.md)
* [DefineSchemaOptions](../interfaces/server.DefineSchemaOptions.md)
* [SystemDataModel](../interfaces/server.SystemDataModel.md)
* [SearchFilterBuilder](../interfaces/server.SearchFilterBuilder.md)
* [SearchFilterFinalizer](../interfaces/server.SearchFilterFinalizer.md)
* [StorageReader](../interfaces/server.StorageReader.md)
* [StorageWriter](../interfaces/server.StorageWriter.md)
* [StorageActionWriter](../interfaces/server.StorageActionWriter.md)
* [VectorSearchQuery](../interfaces/server.VectorSearchQuery.md)
* [VectorFilterBuilder](../interfaces/server.VectorFilterBuilder.md)

## リファレンス \{#references\}

### UserIdentityAttributes \{#useridentityattributes\}

[UserIdentityAttributes](browser.md#useridentityattributes) を再エクスポートします

## 型エイリアス \{#type-aliases\}

### FunctionType \{#functiontype\}

Ƭ **FunctionType**: `"query"` | `"mutation"` | `"action"`

Convex 関数の型。

#### 定義元 \{#defined-in\}

[server/api.ts:19](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L19)

***

### FunctionReference \{#functionreference\}

Ƭ **FunctionReference**&lt;`Type`, `Visibility`, `Args`, `ReturnType`, `ComponentPath`&gt;: `Object`

登録された Convex 関数への参照です。

生成された `api` ユーティリティを使って [FunctionReference](server.md#functionreference) を作成できます。

```js
import { api } from "../convex/_generated/api";

const reference = api.myModule.myFunction;
```

コード生成を使っていない場合は、
[anyApi](server.md#anyapi-1) で参照を作成できます。

```js
import { anyApi } from "convex/server";

const reference = anyApi.myModule.myFunction;
```

関数参照はクライアント側から関数を呼び出すときに利用できます。たとえば、React ではその参照を [useQuery](react.md#usequery) フックに渡すことができます。

```js
const result = useQuery(api.myModule.myFunction);
```

#### 型パラメーター \{#type-parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `Type` | extends [`FunctionType`](server.md#functiontype) | 関数の型（&quot;query&quot;、&quot;mutation&quot;、&quot;action&quot; のいずれか）。 |
| `Visibility` | extends [`FunctionVisibility`](server.md#functionvisibility) = `"public"` | 関数の可視性（&quot;public&quot; または &quot;internal&quot; のいずれか）。 |
| `Args` | extends [`DefaultFunctionArgs`](server.md#defaultfunctionargs) = `any` | この関数の引数。引数名からその型へのマッピングを表すオブジェクトです。 |
| `ReturnType` | `any` | この関数の戻り値の型。 |
| `ComponentPath` | `string` | `undefined` | - |

#### 型定義 \{#type-declaration\}

| 名前 | 型 |
| :------ | :------ |
| `_type` | `Type` |
| `_visibility` | `Visibility` |
| `_args` | `Args` |
| `_returnType` | `ReturnType` |
| `_componentPath` | `ComponentPath` |

#### 定義場所 \{#defined-in\}

[server/api.ts:52](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L52)

***

### ApiFromModules \{#apifrommodules\}

Ƭ **ApiFromModules**&lt;`AllModules`&gt;: [`FilterApi`](server.md#filterapi)&lt;`ApiFromModulesAllowEmptyNodes`&lt;`AllModules`&gt;, [`FunctionReference`](server.md#functionreference)&lt;`any`, `any`, `any`, `any`&gt;&gt;

`convex/` ディレクトリ内のすべてのモジュールの型から、`api` の型を構築します。

`api` は [FunctionReference](server.md#functionreference) を構築するためのユーティリティです。

#### 型パラメーター \{#type-parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `AllModules` | extends `Record`&lt;`string`, `object`&gt; | モジュールパス（例: `"dir/myModule"`）から各モジュールの型へのマッピングを表す型。 |

#### 定義場所 \{#defined-in\}

[server/api.ts:255](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L255)

***

### FilterApi \{#filterapi\}

Ƭ **FilterApi**&lt;`API`, `Predicate`&gt;: [`Expand`](server.md#expand)&lt;&#123; [mod in keyof API as API[mod] extends Predicate ? mod : API[mod] extends FunctionReference&lt;any, any, any, any&gt; ? never : FilterApi&lt;API[mod], Predicate&gt; extends Record&lt;string, never&gt; ? never : mod]: API[mod] extends Predicate ? API[mod] : FilterApi&lt;API[mod], Predicate&gt; &#125;&gt;

Convex のデプロイメントの API オブジェクトから、指定した条件（例: すべての公開クエリ）を満たす関数だけをフィルタリングします。

#### 型パラメーター \{#type-parameters\}

| 名前 |
| :------ |
| `API` |
| `Predicate` |

#### 定義箇所 \{#defined-in\}

[server/api.ts:279](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L279)

***

### AnyApi \{#anyapi\}

Ƭ **AnyApi**: `Record`&lt;`string`, `Record`&lt;`string`, `AnyModuleDirOrFunc`&gt;&gt;

Convex の API オブジェクトが拡張する型です。API を一から定義する場合は、この型を拡張してください。

#### 定義箇所 \{#defined-in\}

[server/api.ts:393](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L393)

***

### PartialApi \{#partialapi\}

Ƭ **PartialApi**&lt;`API`&gt;: &#123; [mod in keyof API]?: API[mod] extends FunctionReference&lt;any, any, any, any&gt; ? API[mod] : PartialApi&lt;API[mod]&gt; &#125;

再帰的な部分 API であり、モックを行う際やカスタム API オブジェクトを構築する際に、API のサブセットを定義するのに役立ちます。

#### 型パラメーター \{#type-parameters\}

| 名前 |
| :------ |
| `API` |

#### 定義元 \{#defined-in\}

[server/api.ts:401](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L401)

***

### FunctionArgs \{#functionargs\}

Ƭ **FunctionArgs**&lt;`FuncRef`&gt;: `FuncRef`[`"_args"`]

[FunctionReference](server.md#functionreference) が与えられたとき、その関数の戻り値の型を取得します。

これは、引数名を値に対応付けるオブジェクトとして表されます。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `FuncRef` | extends `AnyFunctionReference` |

#### 定義場所 \{#defined-in\}

[server/api.ts:435](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L435)

***

### OptionalRestArgs \{#optionalrestargs\}

Ƭ **OptionalRestArgs**&lt;`FuncRef`&gt;: `FuncRef`[`"_args"`] extends `EmptyObject` ? [args?: EmptyObject] : [args: FuncRef[&quot;&#95;args&quot;]]

`FuncRef` の（省略可能な場合もある）引数を表すタプル型です。

この型は、引数を必要としない関数では引数を省略できるようにしつつ、
引数を取るメソッドを型安全にするために使用されます。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `FuncRef` | extends `AnyFunctionReference` |

#### 定義場所 \{#defined-in\}

[server/api.ts:446](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L446)

***

### ArgsAndOptions \{#argsandoptions\}

Ƭ **ArgsAndOptions**&lt;`FuncRef`, `Options`&gt;: `FuncRef`[`"_args"`] extends `EmptyObject` ? [args?: EmptyObject, options?: Options] : [args: FuncRef[&quot;&#95;args&quot;], options?: Options]

`FuncRef` に渡す（省略可能な場合がある）引数のタプル型で、その後に `Options` 型の
options オブジェクトが続きます。

この型は、`useQuery` のようなメソッドを、次のことを許容しつつ型安全に呼び出せるようにするために使われます。

1. 引数を必要としない関数に対して、引数を省略できるようにする。
2. options オブジェクトを省略できるようにする。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `FuncRef` | extends `AnyFunctionReference` |
| `Options` | `Options` |

#### 定義場所 \{#defined-in\}

[server/api.ts:460](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L460)

***

### FunctionReturnType \{#functionreturntype\}

Ƭ **FunctionReturnType**&lt;`FuncRef`&gt;: `FuncRef`[`"_returnType"`]

与えられた [FunctionReference](server.md#functionreference) から、その関数の戻り値の型を取得します。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `FuncRef` | extends `AnyFunctionReference` |

#### 定義元 \{#defined-in\}

[server/api.ts:472](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L472)

***

### AuthConfig \{#authconfig\}

Ƭ **AuthConfig**: `Object`

Convex プロジェクトの `auth.config.ts` でエクスポートされる値です。

```ts
import { AuthConfig } from "convex/server";

export default {
  providers: [
    {
      domain: "https://your.issuer.url.com",
      applicationID: "your-application-id",
    },
  ],
} satisfies AuthConfig;
```

#### 型宣言 \{#type-declaration\}

| 名前 | 型 |
| :------ | :------ |
| `providers` | [`AuthProvider`](server.md#authprovider)[] |

#### 定義場所 \{#defined-in\}

[server/authentication.ts:19](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L19)

***

### AuthProvider \{#authprovider\}

Ƭ **AuthProvider**: &#123; `applicationID`: `string` ; `domain`: `string`  &#125; | &#123; `type`: `"customJwt"` ; `applicationID?`: `string` ; `issuer`: `string` ; `jwks`: `string` ; `algorithm`: `"RS256"` | `"ES256"`  &#125;

あなたのアプリに対して JWT を発行できる認証プロバイダー。

See: https://docs.convex.dev/auth/advanced/custom-auth and https://docs.convex.dev/auth/advanced/custom-jwt

#### 定義元 \{#defined-in\}

[server/authentication.ts:28](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L28)

***

### FunctionHandle \{#functionhandle\}

Ƭ **FunctionHandle**&lt;`Type`, `Args`, `ReturnType`&gt;: `string` &amp; [`FunctionReference`](server.md#functionreference)&lt;`Type`, `"internal"`, `Args`, `ReturnType`&gt;

Convex 関数へのシリアライズ可能な参照。
この参照を別のコンポーネントに渡すと、そのコンポーネントは現在の関数実行中、または後の任意のタイミングで
この関数を呼び出せるようになります。
FunctionHandle は `api.folder.function` 形式の FunctionReference と同様に使われます。
例: `ctx.scheduler.runAfter(0, functionReference, args)`。

関数参照はコードのデプロイをまたいでも安定していますが、
参照先の Convex 関数がすでに存在しない可能性もあります。

これはコンポーネントの機能であり、現在ベータ版です。
この API は安定しておらず、今後のリリースで変更される可能性があります。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | extends [`FunctionType`](server.md#functiontype) |
| `Args` | extends [`DefaultFunctionArgs`](server.md#defaultfunctionargs) = `any` |
| `ReturnType` | `any` |

#### 定義場所 \{#defined-in\}

[server/components/index.ts:35](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L35)

***

### ComponentDefinition \{#componentdefinition\}

Ƭ **ComponentDefinition**&lt;`Exports`&gt;: `Object`

この型のオブジェクトは、コンポーネント定義ディレクトリ内の
convex.config.ts ファイルのデフォルトエクスポートとして指定する必要があります。

これはコンポーネント向けの機能で、現在ベータ版です。
この API は不安定であり、今後のリリースで変更される可能性があります。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Exports` | extends `ComponentExports` = `any` |

#### 型宣言 \{#type-declaration\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `use` | &lt;Definition&gt;(`definition`: `Definition`, `options?`: &#123; `name?`: `string`  &#125;) =&gt; `InstalledComponent`&lt;`Definition`&gt; | このコンポーネント定義に、指定された定義を持つコンポーネントをインストールします。コンポーネント定義と任意の名前を受け取ります。エディタツールではこのメソッドは [ComponentDefinition](server.md#componentdefinition) を受け取ることを前提としていますが、実行時にインポートされるオブジェクトは ImportedComponentDefinition になります。 |
| `__exports` | `Exports` | 提供されるエクスポートを追跡するための内部の型専用プロパティ。**`Deprecated`** これは型専用のプロパティであり、使用しないでください。 |

#### 定義場所 \{#defined-in\}

[server/components/index.ts:84](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L84)

***

### AnyChildComponents \{#anychildcomponents\}

Ƭ **AnyChildComponents**: `Record`&lt;`string`, `AnyComponentReference`&gt;

#### 定義元 \{#defined-in\}

[server/components/index.ts:414](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L414)

***

### AnyComponents \{#anycomponents\}

Ƭ **AnyComponents**: [`AnyChildComponents`](server.md#anychildcomponents)

#### 定義元 \{#defined-in\}

[server/components/index.ts:454](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L454)

***

### GenericDocument \{#genericdocument\}

Ƭ **GenericDocument**: `Record`&lt;`string`, [`Value`](values.md#value)&gt;

Convex に保存されるドキュメント。

#### 定義場所 \{#defined-in\}

[server/data&#95;model.ts:9](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L9)

***

### GenericFieldPaths \{#genericfieldpaths\}

Ƭ **GenericFieldPaths**: `string`

テーブル内のドキュメントに含まれるすべてのフィールドを表す型です。

ここで指定できるのは、フィールド名（例: &quot;name&quot;）か、ネストされたオブジェクト上のフィールドへの参照（例: &quot;properties.name&quot;）のいずれかです。

#### 定義元 \{#defined-in\}

[server/data&#95;model.ts:18](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L18)

***

### GenericIndexFields \{#genericindexfields\}

Ƭ **GenericIndexFields**: `string`[]

インデックス内のフィールドの並び順を表す型です。

フィールド名（例: &quot;name&quot;）か、ネストされたオブジェクト上のフィールドへの参照（例: &quot;properties.name&quot;）のいずれかになります。

#### 定義元 \{#defined-in\}

[server/data&#95;model.ts:29](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L29)

***

### GenericTableIndexes \{#generictableindexes\}

Ƭ **GenericTableIndexes**: `Record`&lt;`string`, [`GenericIndexFields`](server.md#genericindexfields)&gt;

テーブル内のインデックスを表す型です。

各インデックス名を、そのインデックスを構成するフィールドにマッピングするオブジェクトです。

#### 定義場所 \{#defined-in\}

[server/data&#95;model.ts:37](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L37)

***

### GenericSearchIndexConfig \{#genericsearchindexconfig\}

Ƭ **GenericSearchIndexConfig**: `Object`

検索インデックスの設定を記述する型です。

#### 型定義 \{#type-declaration\}

| 名前 | 型 |
| :------ | :------ |
| `searchField` | `string` |
| `filterFields` | `string` |

#### 定義元 \{#defined-in\}

[server/data&#95;model.ts:43](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L43)

***

### GenericTableSearchIndexes \{#generictablesearchindexes\}

Ƭ **GenericTableSearchIndexes**: `Record`&lt;`string`, [`GenericSearchIndexConfig`](server.md#genericsearchindexconfig)&gt;

テーブル内のすべての検索インデックスを表す型です。

各インデックス名を、そのインデックスの設定に対応付けるオブジェクトです。

#### 定義場所 \{#defined-in\}

[server/data&#95;model.ts:54](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L54)

***

### GenericVectorIndexConfig \{#genericvectorindexconfig\}

Ƭ **GenericVectorIndexConfig**: `Object`

ベクターインデックスの構成を表す型。

#### 型の定義 \{#type-declaration\}

| 名前 | 型 |
| :------ | :------ |
| `vectorField` | `string` |
| `dimensions` | `number` |
| `filterFields` | `string` |

#### 定義元 \{#defined-in\}

[server/data&#95;model.ts:63](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L63)

***

### GenericTableVectorIndexes \{#generictablevectorindexes\}

Ƭ **GenericTableVectorIndexes**: `Record`&lt;`string`, [`GenericVectorIndexConfig`](server.md#genericvectorindexconfig)&gt;

テーブル内のすべてのベクトルインデックスを表す型です。

各インデックス名を、そのインデックスの構成情報に対応付けるオブジェクトです。

#### 定義場所 \{#defined-in\}

[server/data&#95;model.ts:75](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L75)

***

### FieldTypeFromFieldPath \{#fieldtypefromfieldpath\}

Ƭ **FieldTypeFromFieldPath**&lt;`Document`, `FieldPath`&gt;: [`FieldTypeFromFieldPathInner`](server.md#fieldtypefromfieldpathinner)&lt;`Document`, `FieldPath`&gt; extends [`Value`](values.md#value) | `undefined` ? [`FieldTypeFromFieldPathInner`](server.md#fieldtypefromfieldpathinner)&lt;`Document`, `FieldPath`&gt; : [`Value`](values.md#value) | `undefined`

ドキュメント内のフィールドの型。

この型は、&quot;name&quot; のような単純なフィールドだけでなく、&quot;properties.name&quot; のようなネストされたフィールドもサポートします。

フィールドがドキュメント内に存在しない場合、そのフィールドは `undefined` と見なされます。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Document` | [`GenericDocument`](server.md#genericdocument) を継承する |
| `FieldPath` | `string` を継承する |

#### 定義場所 \{#defined-in\}

[server/data&#95;model.ts:104](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L104)

***

### FieldTypeFromFieldPathInner \{#fieldtypefromfieldpathinner\}

Ƭ **FieldTypeFromFieldPathInner**&lt;`Document`, `FieldPath`&gt;: `FieldPath` extends `$&#123;infer First&#125;.$&#123;infer Second&#125;` ? `ValueFromUnion`&lt;`Document`, `First`, `Record`&lt;`never`, `never`&gt;&gt; extends infer FieldValue ? `FieldValue` extends [`GenericDocument`](server.md#genericdocument) ? [`FieldTypeFromFieldPath`](server.md#fieldtypefromfieldpath)&lt;`FieldValue`, `Second`&gt; : `undefined` : `undefined` : `ValueFromUnion`&lt;`Document`, `FieldPath`, `undefined`&gt;

[FieldTypeFromFieldPath](server.md#fieldtypefromfieldpath) の内部型です。

一部の TypeScript バージョンではこの型を正しく推論できないため、`Value | undefined` 型に強制するヘルパーでラップされています。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Document` | [`GenericDocument`](server.md#genericdocument) 型を継承 |
| `FieldPath` | `string` 型を継承 |

#### 定義箇所 \{#defined-in\}

[server/data&#95;model.ts:120](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L120)

***

### GenericTableInfo \{#generictableinfo\}

Ƭ **GenericTableInfo**: `Object`

テーブル内で扱うドキュメント型とインデックスを記述する型。

#### 型宣言 \{#type-declaration\}

| 名前 | 型 |
| :------ | :------ |
| `document` | [`GenericDocument`](server.md#genericdocument) |
| `fieldPaths` | [`GenericFieldPaths`](server.md#genericfieldpaths) |
| `indexes` | [`GenericTableIndexes`](server.md#generictableindexes) |
| `searchIndexes` | [`GenericTableSearchIndexes`](server.md#generictablesearchindexes) |
| `vectorIndexes` | [`GenericTableVectorIndexes`](server.md#generictablevectorindexes) |

#### 定義場所 \{#defined-in\}

[server/data&#95;model.ts:145](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L145)

***

### DocumentByInfo \{#documentbyinfo\}

Ƭ **DocumentByInfo**&lt;`TableInfo`&gt;: `TableInfo`[`"document"`]

ある [GenericTableInfo](server.md#generictableinfo) に対応する、テーブル内のドキュメントの型。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableInfo` | extends [`GenericTableInfo`](server.md#generictableinfo) |

#### 定義元 \{#defined-in\}

[server/data&#95;model.ts:157](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L157)

***

### FieldPaths \{#fieldpaths\}

Ƭ **FieldPaths**&lt;`TableInfo`&gt;: `TableInfo`[`"fieldPaths"`]

与えられた [GenericTableInfo](server.md#generictableinfo) に対するテーブル内のフィールドパス。

これはフィールド名（例: &quot;name&quot;）か、入れ子になったオブジェクト上のフィールドを参照するパス（例: &quot;properties.name&quot;）のいずれかになります。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableInfo` | extends [`GenericTableInfo`](server.md#generictableinfo) |

#### 定義場所 \{#defined-in\}

[server/data&#95;model.ts:167](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L167)

***

### インデックス \{#indexes\}

Ƭ **Indexes**&lt;`TableInfo`&gt;: `TableInfo`[`"indexes"`]

指定された [GenericTableInfo](server.md#generictableinfo) に対応するテーブル内のデータベースインデックス。

これは、インデックス名から、そのインデックスを構成するフィールドへのマッピングを表すオブジェクトです。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableInfo` | extends [`GenericTableInfo`](server.md#generictableinfo) |

#### 定義元 \{#defined-in\}

[server/data&#95;model.ts:176](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L176)

***

### IndexNames \{#indexnames\}

Ƭ **IndexNames**&lt;`TableInfo`&gt;: keyof [`Indexes`](server.md#indexes)&lt;`TableInfo`&gt;

特定の [GenericTableInfo](server.md#generictableinfo) に対応するテーブル内のインデックス名。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableInfo` | extends [`GenericTableInfo`](server.md#generictableinfo) |

#### 定義元 \{#defined-in\}

[server/data&#95;model.ts:182](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L182)

***

### NamedIndex \{#namedindex\}

Ƭ **NamedIndex**&lt;`TableInfo`, `IndexName`&gt;: [`Indexes`](server.md#indexes)&lt;`TableInfo`&gt;[`IndexName`]

[GenericTableInfo](server.md#generictableinfo) から、指定された名前のインデックスに含まれるフィールドを抽出します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableInfo` | [`GenericTableInfo`](server.md#generictableinfo) を拡張 |
| `IndexName` | [`IndexNames`](server.md#indexnames)&lt;`TableInfo`&gt; を拡張 |

#### 定義場所 \{#defined-in\}

[server/data&#95;model.ts:189](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L189)

***

### SearchIndexes \{#searchindexes\}

Ƭ **SearchIndexes**&lt;`TableInfo`&gt;: `TableInfo`[`"searchIndexes"`]

指定された [GenericTableInfo](server.md#generictableinfo) における、テーブルに定義された検索インデックスです。

これは、インデックス名を検索インデックス設定に対応付けるオブジェクトです。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableInfo` | [`GenericTableInfo`](server.md#generictableinfo) を拡張 |

#### 定義場所 \{#defined-in\}

[server/data&#95;model.ts:200](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L200)

***

### SearchIndexNames \{#searchindexnames\}

Ƭ **SearchIndexNames**&lt;`TableInfo`&gt;: keyof [`SearchIndexes`](server.md#searchindexes)&lt;`TableInfo`&gt;

指定された [GenericTableInfo](server.md#generictableinfo) のテーブルに存在する検索インデックスの名前。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableInfo` | extends [`GenericTableInfo`](server.md#generictableinfo) |

#### 定義元 \{#defined-in\}

[server/data&#95;model.ts:207](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L207)

***

### NamedSearchIndex \{#namedsearchindex\}

Ƭ **NamedSearchIndex**&lt;`TableInfo`, `IndexName`&gt;: [`SearchIndexes`](server.md#searchindexes)&lt;`TableInfo`&gt;[`IndexName`]

[GenericTableInfo](server.md#generictableinfo) から、名前を指定して検索インデックスの構成を抽出します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableInfo` | [`GenericTableInfo`](server.md#generictableinfo) を拡張 |
| `IndexName` | [`SearchIndexNames`](server.md#searchindexnames)&lt;`TableInfo`&gt; を拡張 |

#### 定義元 \{#defined-in\}

[server/data&#95;model.ts:214](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L214)

***

### VectorIndexes \{#vectorindexes\}

Ƭ **VectorIndexes**&lt;`TableInfo`&gt;: `TableInfo`[`"vectorIndexes"`]

指定された [GenericTableInfo](server.md#generictableinfo) のテーブルにおけるベクターインデックス。

これは、インデックス名をベクターインデックス設定に対応付けるオブジェクトです。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableInfo` | extends [`GenericTableInfo`](server.md#generictableinfo) |

#### 定義場所 \{#defined-in\}

[server/data&#95;model.ts:225](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L225)

***

### VectorIndexNames \{#vectorindexnames\}

Ƭ **VectorIndexNames**&lt;`TableInfo`&gt;: keyof [`VectorIndexes`](server.md#vectorindexes)&lt;`TableInfo`&gt;

指定された [GenericTableInfo](server.md#generictableinfo) のテーブルに定義されているベクターインデックスの名前。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableInfo` | extends [`GenericTableInfo`](server.md#generictableinfo) |

#### 定義元 \{#defined-in\}

[server/data&#95;model.ts:232](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L232)

***

### NamedVectorIndex \{#namedvectorindex\}

Ƭ **NamedVectorIndex**&lt;`TableInfo`, `IndexName`&gt;: [`VectorIndexes`](server.md#vectorindexes)&lt;`TableInfo`&gt;[`IndexName`]

[GenericTableInfo](server.md#generictableinfo) から、名前を指定してベクターインデックスの構成情報を抽出します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableInfo` | extends [`GenericTableInfo`](server.md#generictableinfo) |
| `IndexName` | extends [`VectorIndexNames`](server.md#vectorindexnames)&lt;`TableInfo`&gt; |

#### 定義場所 \{#defined-in\}

[server/data&#95;model.ts:239](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L239)

***

### GenericDataModel \{#genericdatamodel\}

Ƭ **GenericDataModel**: `Record`&lt;`string`, [`GenericTableInfo`](server.md#generictableinfo)&gt;

Convex プロジェクト内のテーブル群を記述する型です。

これは `npx convex dev` によって自動生成されることを想定しています。

#### 定義元 \{#defined-in\}

[server/data&#95;model.ts:252](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L252)

***

### AnyDataModel \{#anydatamodel\}

Ƭ **AnyDataModel**: `Object`

ドキュメントを `any` とみなし、インデックスをサポートしない [GenericDataModel](server.md#genericdatamodel)。

これはスキーマが定義される前に使用されるデフォルトです。

#### インデックスシグネチャ \{#index-signature\}

▪ [tableName: `string`]: &#123; `document`: `any` ; `fieldPaths`: [`GenericFieldPaths`](server.md#genericfieldpaths) ; `indexes`: {} ; `searchIndexes`: {} ; `vectorIndexes`: {}  &#125;

#### 定義元 \{#defined-in\}

[server/data&#95;model.ts:261](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L261)

***

### TableNamesInDataModel \{#tablenamesindatamodel\}

Ƭ **TableNamesInDataModel**&lt;`DataModel`&gt;: keyof `DataModel` &amp; `string`

[GenericDataModel](server.md#genericdatamodel) 内で定義されているすべてのテーブル名を表す型。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](server.md#genericdatamodel) |

#### 定義場所 \{#defined-in\}

[server/data&#95;model.ts:275](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L275)

***

### NamedTableInfo \{#namedtableinfo\}

Ƭ **NamedTableInfo**&lt;`DataModel`, `TableName`&gt;: `DataModel`[`TableName`]

テーブル名を指定して、[GenericDataModel](server.md#genericdatamodel) 内のテーブルの `TableInfo` を抽出します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](server.md#genericdatamodel) |
| `TableName` | extends keyof `DataModel` |

#### 定義場所 \{#defined-in\}

[server/data&#95;model.ts:284](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L284)

***

### DocumentByName \{#documentbyname\}

Ƭ **DocumentByName**&lt;`DataModel`, `TableName`&gt;: `DataModel`[`TableName`][`"document"`]

[GenericDataModel](server.md#genericdatamodel) において、テーブル名に対応するドキュメント型。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | [`GenericDataModel`](server.md#genericdatamodel) を拡張します |
| `TableName` | [`TableNamesInDataModel`](server.md#tablenamesindatamodel)&lt;`DataModel`&gt; を拡張します |

#### 定義元 \{#defined-in\}

[server/data&#95;model.ts:293](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L293)

***

### ExpressionOrValue \{#expressionorvalue\}

Ƭ **ExpressionOrValue**&lt;`T`&gt;: [`Expression`](../classes/server.Expression.md)&lt;`T`&gt; | `T`

[`Expression`](../classes/server.Expression.md) または定数の[値](values.md#value)

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends [`Value`](values.md#value) | `undefined` |

#### 定義元 \{#defined-in\}

[server/filter&#95;builder.ts:38](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L38)

***

### Cursor \{#cursor\}

Ƭ **Cursor**: `string`

データベースクエリのページネーションに使用される不透明な識別子です。

カーソルは [paginate](../interfaces/server.OrderedQuery.md#paginate) から返され、
そのページの結果が終了したクエリ内の位置を表します。

ページネーションを続けるには、[PaginationOptions](../interfaces/server.PaginationOptions.md) オブジェクト内で
カーソルを再度 [paginate](../interfaces/server.OrderedQuery.md#paginate) に渡して、
次の結果ページを取得します。

注意: カーソルは、それが生成されたものと *まったく* 同じデータベースクエリに対してのみ
使用できます。異なるデータベースクエリ間でカーソルを再利用することはできません。

#### 定義場所 \{#defined-in\}

[server/pagination.ts:21](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L21)

***

### GenericMutationCtxWithTable \{#genericmutationctxwithtable\}

Ƭ **GenericMutationCtxWithTable**&lt;`DataModel`&gt;: `Omit`&lt;[`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`DataModel`&gt;, `"db"`&gt; &amp; &#123; `db`: [`GenericDatabaseWriterWithTable`](../interfaces/server.GenericDatabaseWriterWithTable.md)&lt;`DataModel`&gt;  &#125;

Convex のミューテーション関数内で利用できる一連のサービスです。

ミューテーションコンテキストは、サーバー上で実行されるあらゆる Convex のミューテーション
関数に対して、最初の引数として渡されます。

コード生成を利用している場合は、データモデルに合わせて型付けされている
`convex/_generated/server.d.ts` 内の `MutationCtx` 型を使用してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](server.md#genericdatamodel) |

#### 定義元 \{#defined-in\}

[server/registration.ts:109](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L109)

***

### GenericQueryCtxWithTable \{#genericqueryctxwithtable\}

Ƭ **GenericQueryCtxWithTable**&lt;`DataModel`&gt;: `Omit`&lt;[`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`DataModel`&gt;, `"db"`&gt; &amp; &#123; `db`: [`GenericDatabaseReaderWithTable`](../interfaces/server.GenericDatabaseReaderWithTable.md)&lt;`DataModel`&gt;  &#125;

Convex のクエリ関数内で使用するためのサービス群です。

クエリコンテキストは、サーバー側で実行されるあらゆる Convex のクエリ関数に対して、最初の引数として渡されます。

これは、すべてのサービスが読み取り専用である点で MutationCtx と異なります。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](server.md#genericdatamodel) |

#### 定義場所 \{#defined-in\}

[server/registration.ts:167](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L167)

***

### DefaultFunctionArgs \{#defaultfunctionargs\}

Ƭ **DefaultFunctionArgs**: `Record`&lt;`string`, `unknown`&gt;

Convex のクエリ、ミューテーション、またはアクション関数に対するデフォルトの引数の型です。

Convex 関数は常に、引数名を値にマッピングする引数オブジェクトを受け取ります。

#### 定義元 \{#defined-in\}

[server/registration.ts:278](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L278)

***

### ArgsArray \{#argsarray\}

Ƭ **ArgsArray**: `OneArgArray` | `NoArgsArray`

Convex 関数への引数の配列。

Convex 関数は、[DefaultFunctionArgs](server.md#defaultfunctionargs) 型のオブジェクトを 1 つだけ受け取るか、引数を一切受け取らないかのどちらかです。

#### 定義元 \{#defined-in\}

[server/registration.ts:301](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L301)

***

### ArgsArrayToObject \{#argsarraytoobject\}

Ƭ **ArgsArrayToObject**&lt;`Args`&gt;: `Args` extends `OneArgArray`&lt;infer ArgsObject&gt; ? `ArgsObject` : `EmptyObject`

[ArgsArray](server.md#argsarray) を単一のオブジェクト型に変換します。

空の引数配列は `EmptyObject` に変換されます。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Args` | extends [`ArgsArray`](server.md#argsarray) |

#### 定義元 \{#defined-in\}

[server/registration.ts:316](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L316)

***

### FunctionVisibility \{#functionvisibility\}

Ƭ **FunctionVisibility**: `"public"` | `"internal"`

Convex 関数の公開範囲を表す型です。

#### 定義元 \{#defined-in\}

[server/registration.ts:324](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L324)

***

### RegisteredMutation \{#registeredmutation\}

Ƭ **RegisteredMutation**&lt;`Visibility`, `Args`, `Returns`&gt;: &#123; `isConvexFunction`: `true` ; `isMutation`: `true`  &#125; &amp; `VisibilityProperties`&lt;`Visibility`&gt;

このアプリに登録されているミューテーション関数。

関数を [mutationGeneric](server.md#mutationgeneric) または [internalMutationGeneric](server.md#internalmutationgeneric) でラップし、エクスポートすることでミューテーションを作成できます。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Visibility` | [`FunctionVisibility`](server.md#functionvisibility) を拡張する |
| `Args` | [`DefaultFunctionArgs`](server.md#defaultfunctionargs) を拡張する |
| `Returns` | `Returns` |

#### 定義元 \{#defined-in\}

[server/registration.ts:347](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L347)

***

### RegisteredQuery \{#registeredquery\}

Ƭ **RegisteredQuery**&lt;`Visibility`, `Args`, `Returns`&gt;: &#123; `isConvexFunction`: `true` ; `isQuery`: `true`  &#125; &amp; `VisibilityProperties`&lt;`Visibility`&gt;

このアプリに属するクエリ関数です。

関数を [`queryGeneric`](server.md#querygeneric) または [`internalQueryGeneric`](server.md#internalquerygeneric) でラップし、エクスポートすることでクエリを作成できます。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Visibility` | extends [`FunctionVisibility`](server.md#functionvisibility) |
| `Args` | extends [`DefaultFunctionArgs`](server.md#defaultfunctionargs) |
| `Returns` | `Returns` |

#### 定義場所 \{#defined-in\}

[server/registration.ts:376](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L376)

***

### RegisteredAction \{#registeredaction\}

Ƭ **RegisteredAction**&lt;`Visibility`, `Args`, `Returns`&gt;: &#123; `isConvexFunction`: `true` ; `isAction`: `true`  &#125; &amp; `VisibilityProperties`&lt;`Visibility`&gt;

このアプリに登録されているアクションです。

関数を [actionGeneric](server.md#actiongeneric) または [internalActionGeneric](server.md#internalactiongeneric) でラップし、それを `export` することでアクションを作成できます。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Visibility` | [`FunctionVisibility`](server.md#functionvisibility) を拡張 |
| `Args` | [`DefaultFunctionArgs`](server.md#defaultfunctionargs) を拡張 |
| `Returns` | `Returns` |

#### 定義元 \{#defined-in\}

[server/registration.ts:405](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L405)

***

### PublicHttpAction \{#publichttpaction\}

Ƭ **PublicHttpAction**: `Object`

このアプリの公開 API に含まれる HTTP アクションです。

関数を [httpActionGeneric](server.md#httpactiongeneric) でラップしてエクスポートすると、公開 HTTP アクションを作成できます。

#### 型宣言 \{#type-declaration\}

| 名前 | 型 |
| :------ | :------ |
| `isHttp` | `true` |

#### 定義元 \{#defined-in\}

[server/registration.ts:434](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L434)

***

### UnvalidatedFunction \{#unvalidatedfunction\}

Ƭ **UnvalidatedFunction**&lt;`Ctx`, `Args`, `Returns`&gt;: (`ctx`: `Ctx`, ...`args`: `Args`) =&gt; `Returns` | &#123; `handler`: (`ctx`: `Ctx`, ...`args`: `Args`) =&gt; `Returns`  &#125;

**`Deprecated`**

-- Convex 関数を定義する際に使用される型については、
`MutationBuilder` などの型定義を参照してください。

引数のバリデーションを行わない Convex のクエリ、ミューテーション、またはアクション関数の定義です。

Convex 関数は必ず最初の引数としてコンテキストオブジェクトを、
2 番目の引数として（省略可能な）args オブジェクトを受け取ります。

これは次のような関数として記述できます:

```js
import { query } from "./_generated/server";

export const func = query(({ db }, { arg }) => {...});
```

または次のようなオブジェクトで指定します:

```js
import { query } from "./_generated/server";

export const func = query({
  handler: ({ db }, { arg }) => {...},
});
```

引数検証を追加するには [ValidatedFunction](../interfaces/server.ValidatedFunction.md) を参照してください。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Ctx` | `Ctx` |
| `Args` | extends [`ArgsArray`](server.md#argsarray) |
| `Returns` | `Returns` |

#### 定義場所 \{#defined-in\}

[server/registration.ts:472](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L472)

***

### ReturnValueForOptionalValidator \{#returnvalueforoptionalvalidator\}

Ƭ **ReturnValueForOptionalValidator**&lt;`ReturnsValidator`&gt;: [`ReturnsValidator`] extends [[`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt;] ? `ValidatorTypeToReturnType`&lt;[`Infer`](values.md#infer)&lt;`ReturnsValidator`&gt;&gt; : [`ReturnsValidator`] extends [[`PropertyValidators`](values.md#propertyvalidators)] ? `ValidatorTypeToReturnType`&lt;[`ObjectType`](values.md#objecttype)&lt;`ReturnsValidator`&gt;&gt; : `any`

Convex 関数は複数の構文で定義できます。

```
 - query(async (ctx, args) => {...})
 - query({ handler: async (ctx, args) => {...} })
 - query({ args: { a: v.string }, handler: async (ctx, args) => {...} } })
 - query({ args: { a: v.string }, returns: v.string(), handler: async (ctx, args) => {...} } })
```

これらそれぞれの場合に、引数と戻り値の型を正しく推論し、
（与えられていれば）validator から導かれる型を優先したいと考えています。

それぞれについて別々のオーバーロードを用意すると、エラーメッセージにすべて現れてしまうため、
代わりに型パラメータ ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs を使用します。

ReturnValue と OneOrZeroArgs の型は、存在する場合は ArgsValidator と
ReturnsValidator の型によって制約され、さらに関数の引数や戻り値に対する明示的な型注釈からも推論されます。

以下はいくつかのユーティリティ型で、オプションの validator に基づいて
適切な型制約を得るためのものです。

追加のテクニック:

* `Validator | undefined` ではなく `Validator | void` を使うのは、前者が
  `strictNullChecks` 有効時には単なる `Validator` と等価になってしまい、うまく動作しないためです。
* ユニオン型に対する分配を避けるために、長さ 1 のタプル型を使用します
  https://github.com/microsoft/TypeScript/issues/29368#issuecomment-453529532

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `ReturnsValidator` | extends [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) | `void` |

#### 定義場所 \{#defined-in\}

[server/registration.ts:574](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L574)

***

### ArgsArrayForOptionalValidator \{#argsarrayforoptionalvalidator\}

Ƭ **ArgsArrayForOptionalValidator**&lt;`ArgsValidator`&gt;: [`ArgsValidator`] extends [[`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt;] ? `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; : [`ArgsValidator`] extends [[`PropertyValidators`](values.md#propertyvalidators)] ? `OneArgArray`&lt;[`ObjectType`](values.md#objecttype)&lt;`ArgsValidator`&gt;&gt; : [`ArgsArray`](server.md#argsarray)

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `ArgsValidator` | extends [`GenericValidator`](values.md#genericvalidator) | [`PropertyValidators`](values.md#propertyvalidators) | `void` |

#### 定義場所 \{#defined-in\}

[server/registration.ts:582](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L582)

***

### DefaultArgsForOptionalValidator \{#defaultargsforoptionalvalidator\}

Ƭ **DefaultArgsForOptionalValidator**&lt;`ArgsValidator`&gt;: [`ArgsValidator`] extends [[`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt;] ? [[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;] : [`ArgsValidator`] extends [[`PropertyValidators`](values.md#propertyvalidators)] ? [[`ObjectType`](values.md#objecttype)&lt;`ArgsValidator`&gt;] : `OneArgArray`

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `ArgsValidator` | extends [`GenericValidator`](values.md#genericvalidator) | [`PropertyValidators`](values.md#propertyvalidators) | `void` |

#### 定義元 \{#defined-in\}

[server/registration.ts:590](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L590)

***

### MutationBuilder \{#mutationbuilder\}

Ƭ **MutationBuilder**&lt;`DataModel`, `Visibility`&gt;: &lt;ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs&gt;(`mutation`: &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`) =&gt; [`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | [`GenericDataModel`](server.md#genericdatamodel) を拡張 |
| `Visibility` | [`FunctionVisibility`](server.md#functionvisibility) を拡張 |

#### 型宣言 \{#type-declaration\}

▸ &lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`mutation`): [`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Convex のコード生成で使用される内部用の型ヘルパーです。

データモデルに固有の型を [mutationGeneric](server.md#mutationgeneric) に与えるために使用されます。

##### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

##### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `mutation` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

##### 戻り値 \{#returns\}

[`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 定義場所 \{#defined-in\}

[server/registration.ts:604](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L604)

***

### MutationBuilderWithTable \{#mutationbuilderwithtable\}

Ƭ **MutationBuilderWithTable**&lt;`DataModel`, `Visibility`&gt;: &lt;ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs&gt;(`mutation`: &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtxWithTable`](server.md#genericmutationctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtxWithTable`](server.md#genericmutationctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`) =&gt; [`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](server.md#genericdatamodel) |
| `Visibility` | extends [`FunctionVisibility`](server.md#functionvisibility) |

#### 型宣言 \{#type-declaration\}

▸ &lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`mutation`): [`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Convex のコード生成で使用される内部用の型ヘルパーです。

データモデルに特化した型を [mutationGeneric](server.md#mutationgeneric) に付与するために使用されます。

##### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

##### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `mutation` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtxWithTable`](server.md#genericmutationctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtxWithTable`](server.md#genericmutationctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

##### 戻り値 \{#returns\}

[`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 定義元 \{#defined-in\}

[server/registration.ts:697](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L697)

***

### QueryBuilder \{#querybuilder\}

Ƭ **QueryBuilder**&lt;`DataModel`, `Visibility`&gt;: &lt;ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs&gt;(`query`: &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`) =&gt; [`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | [`GenericDataModel`](server.md#genericdatamodel) を拡張 |
| `Visibility` | [`FunctionVisibility`](server.md#functionvisibility) を拡張 |

#### 型宣言 \{#type-declaration\}

▸ &lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`query`): [`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Convex のコード生成で使用される内部用の型ヘルパーです。

データモデルに特化した型を [queryGeneric](server.md#querygeneric) に与えるために使用されます。

##### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

##### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `query` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

##### 戻り値 \{#returns\}

[`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 定義場所 \{#defined-in\}

[server/registration.ts:790](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L790)

***

### QueryBuilderWithTable \{#querybuilderwithtable\}

Ƭ **QueryBuilderWithTable**&lt;`DataModel`, `Visibility`&gt;: &lt;ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs&gt;(`query`: &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtxWithTable`](server.md#genericqueryctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtxWithTable`](server.md#genericqueryctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`) =&gt; [`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](server.md#genericdatamodel) |
| `Visibility` | extends [`FunctionVisibility`](server.md#functionvisibility) |

#### 型宣言 \{#type-declaration\}

▸ &lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`query`): [`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Convex のコード生成で使用される内部用の型ヘルパーです。

データモデルに特化した型を [queryGeneric](server.md#querygeneric) に与えるために使用されます。

##### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

##### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `query` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtxWithTable`](server.md#genericqueryctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtxWithTable`](server.md#genericqueryctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

##### 戻り値 \{#returns\}

[`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 定義元 \{#defined-in\}

[server/registration.ts:879](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L879)

***

### ActionBuilder \{#actionbuilder\}

Ƭ **ActionBuilder**&lt;`DataModel`, `Visibility`&gt;: &lt;ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs&gt;(`func`: &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`) =&gt; [`RegisteredAction`](server.md#registeredaction)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | [`GenericDataModel`](server.md#genericdatamodel) を継承する |
| `Visibility` | [`FunctionVisibility`](server.md#functionvisibility) を継承する |

#### 型宣言 \{#type-declaration\}

▸ &lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`func`): [`RegisteredAction`](server.md#registeredaction)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Convex のコード生成で使用される内部用の型ヘルパーです。

[actionGeneric](server.md#actiongeneric) に、データモデル固有の型を付与するために使用されます。

##### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

##### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `func` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

##### 戻り値 \{#returns\}

[`RegisteredAction`](server.md#registeredaction)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 定義元 \{#defined-in\}

[server/registration.ts:968](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L968)

***

### HttpActionBuilder \{#httpactionbuilder\}

Ƭ **HttpActionBuilder**: (`func`: (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, `request`: `Request`) =&gt; `Promise`&lt;`Response`&gt;) =&gt; [`PublicHttpAction`](server.md#publichttpaction)

#### 型エイリアス宣言 \{#type-declaration\}

▸ (`func`): [`PublicHttpAction`](server.md#publichttpaction)

Convex のコード生成で使用される内部用の型ヘルパーです。

データモデルや関数に特化した型を [httpActionGeneric](server.md#httpactiongeneric) に指定するために使用されます。

##### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `func` | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, `request`: `Request`) =&gt; `Promise`&lt;`Response`&gt; |

##### 戻り値 \{#returns\}

[`PublicHttpAction`](server.md#publichttpaction)

#### 定義場所 \{#defined-in\}

[server/registration.ts:1063](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L1063)

***

### RoutableMethod \{#routablemethod\}

Ƭ **RoutableMethod**: typeof [`ROUTABLE_HTTP_METHODS`](server.md#routable_http_methods)[`number`]

Convex の HTTP アクションでサポートされるメソッドを表す型です。

HEAD は、Convex が GET を実行し、ボディを削除することで処理されます。
CONNECT はサポートされておらず、今後もサポートされません。
TRACE はサポートされておらず、今後もサポートされません。

#### 定義元 \{#defined-in\}

[server/router.ts:31](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L31)

***

### RouteSpecWithPath \{#routespecwithpath\}

Ƭ **RouteSpecWithPath**: `Object`

リクエスト URL のパスを完全一致でマッチさせて、HTTP アクションへのルートを表す型です。

[HttpRouter](../classes/server.HttpRouter.md) によって使用され、リクエストを HTTP アクションへルーティングします。

#### 型定義 \{#type-declaration\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `path` | `string` | ルーティング対象となる正確な HTTP リクエストパス。 |
| `method` | [`RoutableMethod`](server.md#routablemethod) | ルーティング対象の HTTP メソッド（&quot;GET&quot;、&quot;POST&quot;、...）。 |
| `handler` | [`PublicHttpAction`](server.md#publichttpaction) | 実行する HTTP アクション。 |

#### 定義場所 \{#defined-in\}

[server/router.ts:56](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L56)

***

### RouteSpecWithPathPrefix \{#routespecwithpathprefix\}

Ƭ **RouteSpecWithPathPrefix**: `Object`

リクエストの URL パスのプレフィックスマッチを使って HTTP アクションへのルートを表す型。

[HttpRouter](../classes/server.HttpRouter.md) によって使用され、リクエストを HTTP アクションにルーティングします。

#### 型宣言 \{#type-declaration\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `pathPrefix` | `string` | ルーティングする HTTP リクエストパスのプレフィックス。この値で始まるパスを持つリクエストは HTTP アクションにルーティングされます。 |
| `method` | [`RoutableMethod`](server.md#routablemethod) | ルーティングする HTTP メソッド（&quot;GET&quot;、&quot;POST&quot;、...）。 |
| `handler` | [`PublicHttpAction`](server.md#publichttpaction) | 実行する HTTP アクション。 |

#### 定義元 \{#defined-in\}

[server/router.ts:78](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L78)

***

### RouteSpec \{#routespec\}

Ƭ **RouteSpec**: [`RouteSpecWithPath`](server.md#routespecwithpath) | [`RouteSpecWithPathPrefix`](server.md#routespecwithpathprefix)

HTTP アクションへのルートを表す型です。

[HttpRouter](../classes/server.HttpRouter.md) で使用され、HTTP アクションへのリクエストをルーティングします。

#### 定義場所 \{#defined-in\}

[server/router.ts:101](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L101)

***

### SchedulableFunctionReference \{#schedulablefunctionreference\}

Ƭ **SchedulableFunctionReference**: [`FunctionReference`](server.md#functionreference)&lt;`"mutation"` | `"action"`, `"public"` | `"internal"`&gt;

将来の実行をスケジュールできる [FunctionReference](server.md#functionreference)。

スケジュール可能な関数は、`public` または `internal` なミューテーションおよびアクションです。

#### 定義元 \{#defined-in\}

[server/scheduler.ts:11](https://github.com/get-convex/convex-js/blob/main/src/server/scheduler.ts#L11)

***

### GenericSchema \{#genericschema\}

Ƭ **GenericSchema**: `Record`&lt;`string`, [`TableDefinition`](../classes/server.TableDefinition.md)&gt;

Convex プロジェクトのスキーマを表す型です。

これは [defineSchema](server.md#defineschema)、[defineTable](server.md#definetable)、および [v](values.md#v) を使って構築されます。

#### 定義場所 \{#defined-in\}

[server/schema.ts:645](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L645)

***

### DataModelFromSchemaDefinition \{#datamodelfromschemadefinition\}

Ƭ **DataModelFromSchemaDefinition**&lt;`SchemaDef`&gt;: `MaybeMakeLooseDataModel`&lt;&#123; [TableName in keyof SchemaDef[&quot;tables&quot;] &amp; string]: SchemaDef[&quot;tables&quot;][TableName] extends TableDefinition&lt;infer DocumentType, infer Indexes, infer SearchIndexes, infer VectorIndexes&gt; ? Object : never &#125;, `SchemaDef`[`"strictTableNameTypes"`]&gt;

Convex のコード生成で内部的に使用される型です。

[SchemaDefinition](../classes/server.SchemaDefinition.md) を [GenericDataModel](server.md#genericdatamodel) に変換します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `SchemaDef` | extends [`SchemaDefinition`](../classes/server.SchemaDefinition.md)&lt;`any`, `boolean`&gt; |

#### 定義元 \{#defined-in\}

[server/schema.ts:786](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L786)

***

### SystemTableNames \{#systemtablenames\}

Ƭ **SystemTableNames**: [`TableNamesInDataModel`](server.md#tablenamesindatamodel)&lt;[`SystemDataModel`](../interfaces/server.SystemDataModel.md)&gt;

#### 定義場所 \{#defined-in\}

[server/schema.ts:844](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L844)

***

### StorageId \{#storageid\}

Ƭ **StorageId**: `string`

ストレージ内のファイルへの参照。

これは [StorageReader](../interfaces/server.StorageReader.md) と [StorageWriter](../interfaces/server.StorageWriter.md) で使用され、これらにはそれぞれ QueryCtx と MutationCtx を通じて Convex のクエリおよびミューテーション内からアクセスできます。

#### 定義元 \{#defined-in\}

[server/storage.ts:11](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L11)

***

### FileStorageId \{#filestorageid\}

Ƭ **FileStorageId**: [`GenericId`](values.md#genericid)&lt;`"_storage"`&gt; | [`StorageId`](server.md#storageid)

#### 定義元 \{#defined-in\}

[server/storage.ts:12](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L12)

***

### FileMetadata \{#filemetadata\}

Ƭ **FileMetadata**: `Object`

[storage.getMetadata](../interfaces/server.StorageReader.md#getmetadata) によって返される、1 つのファイルのメタデータ。

#### 型定義 \{#type-declaration\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `storageId` | [`StorageId`](server.md#storageid) | ファイルを参照するための ID（例: [storage.getUrl](../interfaces/server.StorageReader.md#geturl) 経由） |
| `sha256` | `string` | ファイル内容の SHA‑256 チェックサム（16 進数エンコード） |
| `size` | `number` | ファイルのサイズ（バイト単位） |
| `contentType` | `string` | `null` | アップロード時に指定されている場合のファイルの Content-Type |

#### 定義場所 \{#defined-in\}

[server/storage.ts:18](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L18)

***

### SystemFields \{#systemfields\}

Ƭ **SystemFields**: `Object`

Convex がドキュメントに自動的に追加するフィールド（`_id` を除く）です。

これは、フィールド名からフィールド型への対応を表すオブジェクト型です。

#### 型定義 \{#type-declaration\}

| 名前 | 型 |
| :------ | :------ |
| `_creationTime` | `number` |

#### 定義元 \{#defined-in\}

[server/system&#95;fields.ts:11](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L11)

***

### IdField \{#idfield\}

Ƭ **IdField**&lt;`TableName`&gt;: `Object`

Convex がドキュメントに自動的に追加する `_id` フィールドです。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 型宣言 \{#type-declaration\}

| 名前 | 型 |
| :------ | :------ |
| `_id` | [`GenericId`](values.md#genericid)&lt;`TableName`&gt; |

#### 定義元 \{#defined-in\}

[server/system&#95;fields.ts:19](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L19)

***

### WithoutSystemFields \{#withoutsystemfields\}

Ƭ **WithoutSystemFields**&lt;`Document`&gt;: [`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;`Document`, keyof [`SystemFields`](server.md#systemfields) | `"_id"`&gt;&gt;

Convex ドキュメントから `_id` や `_creationTime` などのシステムフィールドを除いた型。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Document` | extends [`GenericDocument`](server.md#genericdocument) |

#### 定義箇所 \{#defined-in\}

[server/system&#95;fields.ts:28](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L28)

***

### WithOptionalSystemFields \{#withoptionalsystemfields\}

Ƭ **WithOptionalSystemFields**&lt;`Document`&gt;: [`Expand`](server.md#expand)&lt;[`WithoutSystemFields`](server.md#withoutsystemfields)&lt;`Document`&gt; &amp; `Partial`&lt;`Pick`&lt;`Document`, keyof [`SystemFields`](server.md#systemfields) | `"_id"`&gt;&gt;&gt;

`_id` や `_creationTime` などのシステムフィールドがオプショナルになっている Convex ドキュメント。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Document` | extends [`GenericDocument`](server.md#genericdocument) |

#### 定義場所 \{#defined-in\}

[server/system&#95;fields.ts:37](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L37)

***

### SystemIndexes \{#systemindexes\}

Ƭ **SystemIndexes**: `Object`

Convex がすべてのテーブルに自動的に追加するインデックス。

インデックス名をインデックスのフィールドパスにマッピングするオブジェクトです。

#### 型宣言 \{#type-declaration\}

| 名前 | 型 |
| :------ | :------ |
| `by_id` | [`"_id"`] |
| `by_creation_time` | [`"_creationTime"`] |

#### 定義元 \{#defined-in\}

[server/system&#95;fields.ts:48](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L48)

***

### IndexTiebreakerField \{#indextiebreakerfield\}

Ƭ **IndexTiebreakerField**: `"_creationTime"`

Convex は、他のすべてのフィールドの値が同一である場合の順位付けを行うために、すべてのインデックスの末尾に自動的に &quot;&#95;creationTime&quot; を追加します。

#### 定義場所 \{#defined-in\}

[server/system&#95;fields.ts:61](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L61)

***

### VectorSearch \{#vectorsearch\}

Ƭ **VectorSearch**&lt;`DataModel`, `TableName`, `IndexName`&gt;: (`tableName`: `TableName`, `indexName`: `IndexName`, `query`: [`VectorSearchQuery`](../interfaces/server.VectorSearchQuery.md)&lt;[`NamedTableInfo`](server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;, `IndexName`&gt;) =&gt; `Promise`&lt;&#123; `_id`: [`GenericId`](values.md#genericid)&lt;`TableName`&gt; ; `_score`: `number`  &#125;[]&gt;

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](server.md#genericdatamodel) |
| `TableName` | extends [`TableNamesInDataModel`](server.md#tablenamesindatamodel)&lt;`DataModel`&gt; |
| `IndexName` | extends [`VectorIndexNames`](server.md#vectorindexnames)&lt;[`NamedTableInfo`](server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt; |

#### 型宣言 \{#type-declaration\}

▸ (`tableName`, `indexName`, `query`): `Promise`&lt;&#123; `_id`: [`GenericId`](values.md#genericid)&lt;`TableName`&gt; ; `_score`: `number`  &#125;[]&gt;

##### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `tableName` | `TableName` |
| `indexName` | `IndexName` |
| `query` | [`VectorSearchQuery`](../interfaces/server.VectorSearchQuery.md)&lt;[`NamedTableInfo`](server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;, `IndexName`&gt; |

##### 戻り値 \{#returns\}

`Promise`&lt;&#123; `_id`: [`GenericId`](values.md#genericid)&lt;`TableName`&gt; ; `_score`: `number`  &#125;[]&gt;

#### 定義場所 \{#defined-in\}

[server/vector&#95;search.ts:55](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L55)

***

### Expand \{#expand\}

Ƭ **Expand**&lt;`ObjectType`&gt;: `ObjectType` extends `Record`&lt;`any`, `any`&gt; ? &#123; [Key in keyof ObjectType]: ObjectType[Key] &#125; : `never`

ちょっとしたハックです！この型は、TypeScript がオブジェクト型を表示する方法を単純化します。

機能的にはオブジェクト型に対する恒等関数ですが、実際には `A & B` のような式を
単純化できます。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `ObjectType` | extends `Record`&lt;`any`, `any`&gt; |

#### 定義元 \{#defined-in\}

[type&#95;utils.ts:12](https://github.com/get-convex/convex-js/blob/main/src/type_utils.ts#L12)

***

### BetterOmit \{#betteromit\}

Ƭ **BetterOmit**&lt;`T`, `K`&gt;: &#123; [Property in keyof T as Property extends K ? never : Property]: T[Property] &#125;

次のような `Omit<>` 型です:

1. ユニオン型の各要素に対して適用される。
2. 基になる型のインデックスシグネチャを保持する。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | `T` |
| `K` | extends keyof `T` |

#### 定義元 \{#defined-in\}

[type&#95;utils.ts:24](https://github.com/get-convex/convex-js/blob/main/src/type_utils.ts#L24)

## 変数 \{#variables\}

### anyApi \{#anyapi\}

• `Const` **anyApi**: [`AnyApi`](server.md#anyapi)

コード生成を利用していないプロジェクトで [FunctionReference](server.md#functionreference) を構築するためのユーティリティです。

関数への参照は次のように作成できます：

```js
const reference = anyApi.myModule.myFunction;
```

これは、プロジェクト内にどのようなディレクトリやモジュールが存在していても、任意のパスにアクセスできるようにします。すべての関数参照は
AnyFunctionReference として型付けされます。

コード生成を使用している場合は、代わりに `convex/_generated/api` の `api`
を使用してください。その方が型安全性が高く、エディターでより良いオートコンプリートが得られます。

#### 定義場所 \{#defined-in\}

[server/api.ts:427](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L427)

***

### paginationOptsValidator \{#paginationoptsvalidator\}

• `Const` **paginationOptsValidator**: [`VObject`](../classes/values.VObject.md)&lt;&#123; `id`: `undefined` | `number` ; `endCursor`: `undefined` | `null` | `string` ; `maximumRowsRead`: `undefined` | `number` ; `maximumBytesRead`: `undefined` | `number` ; `numItems`: `number` ; `cursor`: `null` | `string`  &#125;, &#123; `numItems`: [`VFloat64`](../classes/values.VFloat64.md)&lt;`number`, `"required"`&gt; ; `cursor`: [`VUnion`](../classes/values.VUnion.md)&lt;`null` | `string`, [[`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"required"`, `never`&gt; ; `endCursor`: [`VUnion`](../classes/values.VUnion.md)&lt;`undefined` | `null` | `string`, [[`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"optional"`, `never`&gt; ; `id`: [`VFloat64`](../classes/values.VFloat64.md)&lt;`undefined` | `number`, `"optional"`&gt; ; `maximumRowsRead`: [`VFloat64`](../classes/values.VFloat64.md)&lt;`undefined` | `number`, `"optional"`&gt; ; `maximumBytesRead`: [`VFloat64`](../classes/values.VFloat64.md)&lt;`undefined` | `number`, `"optional"`&gt;  &#125;, `"required"`, `"id"` | `"numItems"` | `"cursor"` | `"endCursor"` | `"maximumRowsRead"` | `"maximumBytesRead"`&gt;

[PaginationOptions](../interfaces/server.PaginationOptions.md) 向けの [Validator](values.md#validator)。

標準の [PaginationOptions](../interfaces/server.PaginationOptions.md) プロパティに加えて、[usePaginatedQuery](react.md#usepaginatedquery) で使用されるキャッシュバスター用の省略可能な `id` プロパティを含みます。

#### 定義場所 \{#defined-in\}

[server/pagination.ts:133](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L133)

***

### ROUTABLE_HTTP_METHODS \{#routable_http_methods\}

• `Const` **ROUTABLE&#95;HTTP&#95;METHODS**: readonly [`"GET"`, `"POST"`, `"PUT"`, `"DELETE"`, `"OPTIONS"`, `"PATCH"`]

Convex HTTP アクションでサポートされているメソッドの一覧です。

HEAD は、GET を実行し、ボディを削除することで Convex によって処理されます。
CONNECT はサポートされておらず、今後もサポートされる予定はありません。
TRACE はサポートされておらず、今後もサポートされる予定はありません。

#### 定義場所 \{#defined-in\}

[server/router.ts:14](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L14)

## 関数 \{#functions\}

### getFunctionName \{#getfunctionname\}

▸ **getFunctionName**(`functionReference`): `string`

[FunctionReference](server.md#functionreference) から関数名を取得します。

関数名は &quot;myDir/myModule:myFunction&quot; のような文字列です。関数のエクスポート名が `"default"` の場合は、関数名部分が省略されます（例: &quot;myDir/myModule&quot;）。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `functionReference` | `AnyFunctionReference` | 名前を取得するための [FunctionReference](server.md#functionreference)。 |

#### 返り値 \{#returns\}

`string`

関数名を表す文字列です。

#### 定義元 \{#defined-in\}

[server/api.ts:78](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L78)

***

### makeFunctionReference \{#makefunctionreference\}

▸ **makeFunctionReference**&lt;`type`, `args`, `ret`&gt;(`name`): [`FunctionReference`](server.md#functionreference)&lt;`type`, `"public"`, `args`, `ret`&gt;

`FunctionReference` は通常、生成されたコードから提供されますが、カスタムクライアントでは
手動で構築できると便利な場合があります。

実際の関数参照は実行時には空のオブジェクトですが、同じインターフェースを、
コード生成を使わないテストやクライアント向けにオブジェクトとして実装することもできます。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `type` | extends [`FunctionType`](server.md#functiontype) |
| `args` | extends [`DefaultFunctionArgs`](server.md#defaultfunctionargs) = `any` |
| `ret` | `any` |

#### パラメータ \{#parameters\}

| Name | Type | 説明 |
| :------ | :------ | :------ |
| `name` | `string` | 関数の識別子です。例: `path/to/file:functionName` |

#### 戻り値 \{#returns\}

[`FunctionReference`](server.md#functionreference)&lt;`type`, `"public"`, `args`, `ret`&gt;

#### 定義場所 \{#defined-in\}

[server/api.ts:122](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L122)

***

### filterApi \{#filterapi\}

▸ **filterApi**&lt;`API`, `Predicate`&gt;(`api`): [`FilterApi`](server.md#filterapi)&lt;`API`, `Predicate`&gt;

型 `API` の `api` と FunctionReference のサブタイプを受け取り、一致する関数参照のみを含む `api` オブジェクトを返します。

```ts
const q = filterApi<typeof api, FunctionReference<"query">>(api)
```

#### 型パラメーター \{#type-parameters\}

| 名前 |
| :------ |
| `API` |
| `Predicate` |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `api` | `API` |

#### 戻り値 \{#returns\}

[`FilterApi`](server.md#filterapi)&lt;`API`, `Predicate`&gt;

#### 定義元 \{#defined-in\}

[server/api.ts:301](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L301)

***

### createFunctionHandle \{#createfunctionhandle\}

▸ **createFunctionHandle**&lt;`Type`, `Args`, `ReturnType`&gt;(`functionReference`): `Promise`&lt;[`FunctionHandle`](server.md#functionhandle)&lt;`Type`, `Args`, `ReturnType`&gt;&gt;

Convex 関数へのシリアライズ可能な参照を作成します。
この参照を別のコンポーネントに渡すことで、そのコンポーネントは現在の関数実行中、またはその後の任意のタイミングで
その関数を呼び出せるようになります。
Function handle は `api.folder.function` のような FunctionReference と同様に使用されます。
例: `ctx.scheduler.runAfter(0, functionReference, args)`。

関数参照はコードの push をまたいでも安定していますが、参照先の Convex 関数がすでに存在しない可能性もあります。

これはコンポーネントの機能の一部であり、ベータ版です。
この API はまだ安定しておらず、今後のリリースで変更される可能性があります。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | extends [`FunctionType`](server.md#functiontype) |
| `Args` | extends [`DefaultFunctionArgs`](server.md#defaultfunctionargs) |
| `ReturnType` | `ReturnType` |

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `functionReference` | [`FunctionReference`](server.md#functionreference)&lt;`Type`, `"public"` | `"internal"`, `Args`, `ReturnType`&gt; |

#### 戻り値 \{#returns\}

`Promise`&lt;[`FunctionHandle`](server.md#functionhandle)&lt;`Type`, `Args`, `ReturnType`&gt;&gt;

#### 定義元 \{#defined-in\}

[server/components/index.ts:54](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L54)

***

### defineComponent \{#definecomponent\}

▸ **defineComponent**&lt;`Exports`&gt;(`name`): [`ComponentDefinition`](server.md#componentdefinition)&lt;`Exports`&gt;

Convex のデプロイメント内で、名前空間付きリソースから構成されるコンポーネントを定義します。

&quot;cool-component/convex.config.js&quot; のようなモジュールのデフォルトエクスポートは
通常は `@link ComponentDefinition}` ですが、コンポーネント定義の評価中は
代わりにこの型になります。

@param name name には英数字とアンダースコアのみを使用できます。一般的には
`"onboarding_flow_tracker"` のように、小文字とアンダースコアで命名します。

これはコンポーネント向けの機能であり、現在ベータ版です。
この API は安定しておらず、今後のリリースで変更される可能性があります。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Exports` | extends `ComponentExports` = `any` |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `name` | `string` |

#### 戻り値 \{#returns\}

[`ComponentDefinition`](server.md#componentdefinition)&lt;`Exports`&gt;

#### 定義元 \{#defined-in\}

[server/components/index.ts:371](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L371)

***

### defineApp \{#defineapp\}

▸ **defineApp**(): `AppDefinition`

Convex デプロイメントの再利用可能な構成要素であるコンポーネントを、この Convex アプリにアタッチします。

これはコンポーネント機能の一部であり、現在ベータ版です。
この API は不安定であり、今後のリリースで変更される可能性があります。

#### 戻り値 \{#returns\}

`AppDefinition`

#### 定義場所 \{#defined-in\}

[server/components/index.ts:397](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L397)

***

### componentsGeneric \{#componentsgeneric\}

▸ **componentsGeneric**(): [`AnyChildComponents`](server.md#anychildcomponents)

#### 戻り値 \{#returns\}

[`AnyChildComponents`](server.md#anychildcomponents)

#### 定義場所 \{#defined-in\}

[server/components/index.ts:452](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L452)

***

### getFunctionAddress \{#getfunctionaddress\}

▸ **getFunctionAddress**(`functionReference`): &#123; `functionHandle`: `string` = functionReference; `name?`: `undefined` ; `reference?`: `undefined` = referencePath &#125; | &#123; `functionHandle?`: `undefined` = functionReference; `name`: `any` ; `reference?`: `undefined` = referencePath &#125; | &#123; `functionHandle?`: `undefined` = functionReference; `name?`: `undefined` ; `reference`: `string` = referencePath &#125;

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `functionReference` | `any` |

#### 戻り値 \{#returns\}

&#123; `functionHandle`: `string` = functionReference; `name?`: `undefined` ; `reference?`: `undefined` = referencePath &#125; | &#123; `functionHandle?`: `undefined` = functionReference; `name`: `any` ; `reference?`: `undefined` = referencePath &#125; | &#123; `functionHandle?`: `undefined` = functionReference; `name?`: `undefined` ; `reference`: `string` = referencePath &#125;

#### 定義箇所 \{#defined-in\}

[server/components/paths.ts:20](https://github.com/get-convex/convex-js/blob/main/src/server/components/paths.ts#L20)

***

### cronJobs \{#cronjobs\}

▸ **cronJobs**(): [`Crons`](../classes/server.Crons.md)

定期実行タスクをスケジュールする CronJobs オブジェクトを作成します。

```js
// convex/crons.js
import { cronJobs } from 'convex/server';
import { api } from "./_generated/api";

const crons = cronJobs();
crons.weekly(
  "weekly re-engagement email",
  {
    hourUTC: 17, // (太平洋標準時午前9:30/太平洋夏時間午前10:30)
    minuteUTC: 30,
  },
  api.emails.send
)
export default crons;
```

#### 戻り値 \{#returns\}

[`Crons`](../classes/server.Crons.md)

#### 定義場所 \{#defined-in\}

[server/cron.ts:180](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L180)

***

### mutationGeneric \{#mutationgeneric\}

▸ **mutationGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`mutation`): [`RegisteredMutation`](server.md#registeredmutation)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

この Convex アプリのパブリック API におけるミューテーションを定義します。

この関数は Convex データベースを変更する権限を持ち、クライアントからアクセス可能になります。

コード生成を使用している場合は、データモデル向けに型付けされている
`convex/_generated/server.d.ts` 内の `mutation` 関数を使用してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `mutation` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

#### 戻り値 \{#returns\}

[`RegisteredMutation`](server.md#registeredmutation)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

ラップされたミューテーション。これを `export` して名前を付け、他の場所から参照できるようにします。

#### 定義場所 \{#defined-in\}

[server/registration.ts:608](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L608)

***

### internalMutationGeneric \{#internalmutationgeneric\}

▸ **internalMutationGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`mutation`): [`RegisteredMutation`](server.md#registeredmutation)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

他の Convex 関数からのみアクセス可能な（クライアントからはアクセスできない）ミューテーションを定義します。

この関数は Convex データベースを変更できます。クライアントからはアクセスできません。

コード生成を使用している場合は、データモデルに対して型付けされている
`convex/_generated/server.d.ts` 内の `internalMutation` 関数を使用してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `mutation` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

#### 戻り値 \{#returns\}

[`RegisteredMutation`](server.md#registeredmutation)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

ラップされたミューテーション。この関数を `export` して名前を付け、アクセス可能にします。

#### 定義元 \{#defined-in\}

[server/registration.ts:608](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L608)

***

### queryGeneric \{#querygeneric\}

▸ **queryGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`query`): [`RegisteredQuery`](server.md#registeredquery)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

この Convex アプリのパブリック API にクエリを定義します。

この関数は Convex データベースを読み取ることができ、クライアントからアクセスできるようになります。

コード生成を使用している場合は、データモデルに合わせて型付けされている
`convex/_generated/server.d.ts` 内の `query` 関数を使用してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `query` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

#### 戻り値 \{#returns\}

[`RegisteredQuery`](server.md#registeredquery)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

ラップされたクエリ。これを `export` して名前を付け、アクセス可能にします。

#### 定義場所 \{#defined-in\}

[server/registration.ts:794](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L794)

***

### internalQueryGeneric \{#internalquerygeneric\}

▸ **internalQueryGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`query`): [`RegisteredQuery`](server.md#registeredquery)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

他の Convex 関数からのみ呼び出すことができる（クライアントからは呼び出せない）クエリを定義します。

この関数は Convex データベースからの読み取りを行えます。クライアントからはアクセスできません。

コード生成を使用している場合は、データモデルに対して型付けされた `convex/_generated/server.d.ts` 内の `internalQuery` 関数を使用してください。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `query` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

#### 戻り値 \{#returns\}

[`RegisteredQuery`](server.md#registeredquery)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

ラップされたクエリです。これを `export` として定義して名前を付け、利用可能にします。

#### 定義元 \{#defined-in\}

[server/registration.ts:794](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L794)

***

### actionGeneric \{#actiongeneric\}

▸ **actionGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`func`): [`RegisteredAction`](server.md#registeredaction)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

この Convex アプリのパブリック API で公開されるアクションを定義します。

コード生成を利用している場合は、データモデルに型付けされた `convex/_generated/server.d.ts` 内の `action` 関数を使用してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### パラメータ \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `func` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` | 対象の関数です。最初の引数として [GenericActionCtx](../interfaces/server.GenericActionCtx.md) を受け取ります。 |

#### 戻り値 \{#returns\}

[`RegisteredAction`](server.md#registeredaction)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

ラップされた関数。これを `export` して名前を付け、アクセス可能にします。

#### 定義場所 \{#defined-in\}

[server/registration.ts:972](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L972)

***

### internalActionGeneric \{#internalactiongeneric\}

▸ **internalActionGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`func`): [`RegisteredAction`](server.md#registeredaction)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

他の Convex 関数からのみ呼び出せるアクションを定義します（クライアントからはアクセスできません）。

コード生成を使用している場合は、データモデルに対して型付けされた
`convex/_generated/server.d.ts` 内の `internalAction` 関数を使用してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### パラメータ \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `func` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` | 関数です。この関数は最初の引数として [GenericActionCtx](../interfaces/server.GenericActionCtx.md) を受け取ります。 |

#### 戻り値 \{#returns\}

[`RegisteredAction`](server.md#registeredaction)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

ラップされた関数。これを `export` して名前を付け、他のモジュールから参照できるようにします。

#### 定義元 \{#defined-in\}

[server/registration.ts:972](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L972)

***

### httpActionGeneric \{#httpactiongeneric\}

▸ **httpActionGeneric**(`func`): [`PublicHttpAction`](server.md#publichttpaction)

Convex の HTTP アクションを定義します。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `func` | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;[`GenericDataModel`](server.md#genericdatamodel)&gt;, `request`: `Request`) =&gt; `Promise`&lt;`Response`&gt; | 関数。第1引数として [GenericActionCtx](../interfaces/server.GenericActionCtx.md)、第2引数として `Request` オブジェクトを受け取ります。 |

#### 戻り値 \{#returns\}

[`PublicHttpAction`](server.md#publichttpaction)

ラップされた関数です。`convex/http.js` 内でこの関数に対応する URL パスを設定してください。

#### 定義場所 \{#defined-in\}

[server/impl/registration&#95;impl.ts:467](https://github.com/get-convex/convex-js/blob/main/src/server/impl/registration_impl.ts#L467)

***

### paginationResultValidator \{#paginationresultvalidator\}

▸ **paginationResultValidator**&lt;`T`&gt;(`itemValidator`): [`VObject`](../classes/values.VObject.md)&lt;&#123; `splitCursor`: `undefined` | `null` | `string` ; `pageStatus`: `undefined` | `null` | `"SplitRecommended"` | `"SplitRequired"` ; `page`: `T`[`"type"`][] ; `continueCursor`: `string` ; `isDone`: `boolean`  &#125;, &#123; `page`: [`VArray`](../classes/values.VArray.md)&lt;`T`[`"type"`][], `T`, `"required"`&gt; ; `continueCursor`: [`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt; ; `isDone`: [`VBoolean`](../classes/values.VBoolean.md)&lt;`boolean`, `"required"`&gt; ; `splitCursor`: [`VUnion`](../classes/values.VUnion.md)&lt;`undefined` | `null` | `string`, [[`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"optional"`, `never`&gt; ; `pageStatus`: [`VUnion`](../classes/values.VUnion.md)&lt;`undefined` | `null` | `"SplitRecommended"` | `"SplitRequired"`, [[`VLiteral`](../classes/values.VLiteral.md)&lt;`"SplitRecommended"`, `"required"`&gt;, [`VLiteral`](../classes/values.VLiteral.md)&lt;`"SplitRequired"`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"optional"`, `never`&gt;  &#125;, `"required"`, `"page"` | `"continueCursor"` | `"isDone"` | `"splitCursor"` | `"pageStatus"`&gt;

[PaginationResult](../interfaces/server.PaginationResult.md) 用の [Validator](values.md#validator) ファクトリ関数。

指定したアイテム用のバリデーターを使って、[paginate](../interfaces/server.OrderedQuery.md#paginate) 呼び出し結果用のバリデーターを作成します。

例：

```ts
const paginationResultValidator = paginationResultValidator(v.object({
  _id: v.id("users"),
  _creationTime: v.number(),
  name: v.string(),
}));
```

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends [`Validator`](values.md#validator)&lt;[`Value`](values.md#value), `"required"`, `string`&gt; |

#### パラメータ \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `itemValidator` | `T` | ページ内の各アイテムを検証するバリデータ |

#### 戻り値 \{#returns\}

[`VObject`](../classes/values.VObject.md)&lt;&#123; `splitCursor`: `undefined` | `null` | `string` ; `pageStatus`: `undefined` | `null` | `"SplitRecommended"` | `"SplitRequired"` ; `page`: `T`[`"type"`][] ; `continueCursor`: `string` ; `isDone`: `boolean`  &#125;, &#123; `page`: [`VArray`](../classes/values.VArray.md)&lt;`T`[`"type"`][], `T`, `"required"`&gt; ; `continueCursor`: [`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt; ; `isDone`: [`VBoolean`](../classes/values.VBoolean.md)&lt;`boolean`, `"required"`&gt; ; `splitCursor`: [`VUnion`](../classes/values.VUnion.md)&lt;`undefined` | `null` | `string`, [[`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"optional"`, `never`&gt; ; `pageStatus`: [`VUnion`](../classes/values.VUnion.md)&lt;`undefined` | `null` | `"SplitRecommended"` | `"SplitRequired"`, [[`VLiteral`](../classes/values.VLiteral.md)&lt;`"SplitRecommended"`, `"required"`&gt;, [`VLiteral`](../classes/values.VLiteral.md)&lt;`"SplitRequired"`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"optional"`, `never`&gt;  &#125;, `"required"`, `"page"` | `"continueCursor"` | `"isDone"` | `"splitCursor"` | `"pageStatus"`&gt;

ページネーション結果のバリデータ

#### 定義場所 \{#defined-in\}

[server/pagination.ts:162](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L162)

***

### httpRouter \{#httprouter\}

▸ **httpRouter**(): [`HttpRouter`](../classes/server.HttpRouter.md)

新しい [`HttpRouter`](../classes/server.HttpRouter.md) オブジェクトを返します。

#### 返り値 \{#returns\}

[`HttpRouter`](../classes/server.HttpRouter.md)

#### 定義元 \{#defined-in\}

[server/router.ts:47](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L47)

***

### defineTable \{#definetable\}

▸ **defineTable**&lt;`DocumentSchema`&gt;(`documentSchema`): [`TableDefinition`](../classes/server.TableDefinition.md)&lt;`DocumentSchema`&gt;

スキーマ内にテーブルを定義します。

ドキュメントのスキーマは、次のようなオブジェクトとして指定することもできます

```ts
defineTable({
  field: v.string()
});
```

または、次のようなスキーマ型として指定できます:

```ts
defineTable(
 v.union(
   v.object({...}),
   v.object({...})
 )
);
```

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DocumentSchema` | extends [`Validator`](values.md#validator)&lt;`Record`&lt;`string`, `any`&gt;, `"required"`, `any`&gt; |

#### パラメーター \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `documentSchema` | `DocumentSchema` | このテーブルに格納されるドキュメントの型。 |

#### 戻り値 \{#returns\}

[`TableDefinition`](../classes/server.TableDefinition.md)&lt;`DocumentSchema`&gt;

テーブルの [TableDefinition](../classes/server.TableDefinition.md)。

#### 定義場所 \{#defined-in\}

[server/schema.ts:593](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L593)

▸ **defineTable**&lt;`DocumentSchema`&gt;(`documentSchema`): [`TableDefinition`](../classes/server.TableDefinition.md)&lt;[`VObject`](../classes/values.VObject.md)&lt;[`ObjectType`](values.md#objecttype)&lt;`DocumentSchema`&gt;, `DocumentSchema`&gt;&gt;

スキーマ内でテーブルを定義します。

ドキュメントのスキーマは、次のようなオブジェクトとして指定できます

```ts
defineTable({
  field: v.string()
});
```

またはスキーマ型として次のように

```ts
defineTable(
 v.union(
   v.object({...}),
   v.object({...})
 )
);
```

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DocumentSchema` | extends `Record`&lt;`string`, [`GenericValidator`](values.md#genericvalidator)&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `documentSchema` | `DocumentSchema` | このテーブルに保存されるドキュメントの型です。 |

#### 戻り値 \{#returns\}

[`TableDefinition`](../classes/server.TableDefinition.md)&lt;[`VObject`](../classes/values.VObject.md)&lt;[`ObjectType`](values.md#objecttype)&lt;`DocumentSchema`&gt;, `DocumentSchema`&gt;&gt;

テーブル用の [TableDefinition](../classes/server.TableDefinition.md) です。

#### 定義元 \{#defined-in\}

[server/schema.ts:621](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L621)

***

### defineSchema \{#defineschema\}

▸ **defineSchema**&lt;`Schema`, `StrictTableNameTypes`&gt;(`schema`, `options?`): [`SchemaDefinition`](../classes/server.SchemaDefinition.md)&lt;`Schema`, `StrictTableNameTypes`&gt;

この Convex プロジェクトのスキーマを定義します。

これは `convex/` ディレクトリ内の `schema.ts` ファイルから、次のようにエクスポートしてください:

```ts
export default defineSchema({
  ...
});
```

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Schema` | extends [`GenericSchema`](server.md#genericschema) |
| `StrictTableNameTypes` | extends `boolean` = `true` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `schema` | `Schema` | このプロジェクト内のすべてのテーブルについて、テーブル名をキーとして [TableDefinition](../classes/server.TableDefinition.md) を値に持つマップです。 |
| `options?` | [`DefineSchemaOptions`](../interfaces/server.DefineSchemaOptions.md)&lt;`StrictTableNameTypes`&gt; | 省略可能な設定です。詳細は [DefineSchemaOptions](../interfaces/server.DefineSchemaOptions.md) を参照してください。 |

#### 戻り値 \{#returns\}

[`SchemaDefinition`](../classes/server.SchemaDefinition.md)&lt;`Schema`, `StrictTableNameTypes`&gt;

スキーマを返します。

#### 定義元 \{#defined-in\}

[server/schema.ts:769](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L769)