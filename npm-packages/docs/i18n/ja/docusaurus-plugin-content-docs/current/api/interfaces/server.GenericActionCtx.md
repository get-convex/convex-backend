---
id: "server.GenericActionCtx"
title: "インターフェイス: GenericActionCtx<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericActionCtx

Convex のアクション関数内で使用するためのサービス群。

このコンテキストは、サーバー上で実行される任意の Convex アクションに
最初の引数として渡されます。

コード生成を使用している場合は、データモデルに対して型付けされた
`convex/_generated/server.d.ts` 内の `ActionCtx` 型を使用してください。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## プロパティ \{#properties\}

### scheduler \{#scheduler\}

• **scheduler**: [`Scheduler`](server.Scheduler.md)

Convex 関数を将来のタイミングで実行するためにスケジュールするユーティリティです。

#### 定義元 \{#defined-in\}

[server/registration.ts:236](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L236)

***

### auth \{#auth\}

• **auth**: [`Auth`](server.Auth.md)

現在認証されているユーザーに関する情報。

#### 定義場所 \{#defined-in\}

[server/registration.ts:241](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L241)

***

### storage \{#storage\}

• **storage**: [`StorageActionWriter`](server.StorageActionWriter.md)

ストレージに保存されたファイルの読み書きを行うためのユーティリティです。

#### 定義元 \{#defined-in\}

[server/registration.ts:246](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L246)

## メソッド \{#methods\}

### runQuery \{#runquery\}

▸ **runQuery**&lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

指定された名前と引数で Convex クエリを実行します。

ユーザーがこのクエリを直接呼び出せないようにするには、`internalQuery` の使用を検討してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`, `"public"` | `"internal"`&gt; |

#### パラメータ \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | 実行するクエリを指定するための [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; | クエリ関数に渡す引数。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

クエリの結果を返す Promise。

#### 定義元 \{#defined-in\}

[server/registration.ts:196](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L196)

***

### runMutation \{#runmutation\}

▸ **runMutation**&lt;`Mutation`&gt;(`mutation`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

指定された名前と引数を使って Convex のミューテーションを実行します。

ユーザーがミューテーションを直接呼び出せないようにするために、`internalMutation` の使用を検討してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`, `"public"` | `"internal"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `mutation` | `Mutation` | 実行するミューテーションの [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Mutation`&gt; | ミューテーション関数に渡す引数。 |

#### Returns \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

ミューテーションの結果が解決される Promise。

#### 定義元 \{#defined-in\}

[server/registration.ts:211](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L211)

***

### runAction \{#runaction\}

▸ **runAction**&lt;`Action`&gt;(`action`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

指定された名前と引数で Convex アクションを実行します。

ユーザーがアクションを直接呼び出せないようにするために、`internalAction` の使用を検討してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"action"`, `"public"` | `"internal"`&gt; |

#### パラメーター \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `action` | `Action` | 実行するアクションを示す [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Action`&gt; | アクション関数に渡す引数。 |

#### 返り値 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

アクションの結果を返す Promise。

#### 定義元 \{#defined-in\}

[server/registration.ts:228](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L228)

***

### vectorSearch \{#vectorsearch\}

▸ **vectorSearch**&lt;`TableName`, `IndexName`&gt;(`tableName`, `indexName`, `query`): `Promise`&lt;&#123; `_id`: [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; ; `_score`: `number`  &#125;[]&gt;

指定したテーブルとインデックスでベクター検索を実行します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |
| `IndexName` | extends `string` | `number` | `symbol` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `tableName` | `TableName` | クエリを実行するテーブルの名前。 |
| `indexName` | `IndexName` | クエリを実行するテーブル上のベクトルインデックスの名前。 |
| `query` | `Object` | クエリ対象のベクトル、返す結果数、およびフィルターを含む [VectorSearchQuery](server.VectorSearchQuery.md)。 |
| `query.vector` | `number`[] | クエリベクトル。これはインデックスの `dimensions` と同じ長さでなければなりません。このベクトル検索では、このベクトルに最も類似したドキュメントの ID を返します。 |
| `query.limit?` | `number` | 返す結果数。指定する場合は 1 以上 256 以下である必要があります。 **`Default`** `ts 10 ` |
| `query.filter?` | (`q`: [`VectorFilterBuilder`](server.VectorFilterBuilder.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;, [`NamedVectorIndex`](../modules/server.md#namedvectorindex)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;, `IndexName`&gt;&gt;) =&gt; [`FilterExpression`](../classes/server.FilterExpression.md)&lt;`boolean`&gt; | インデックスのフィルターフィールドに対して動作する `q.or` と `q.eq` から構成されるオプションのフィルター式。例: `filter: q => q.or(q.eq("genre", "comedy"), q.eq("genre", "drama"))` |

#### 戻り値 \{#returns\}

`Promise`&lt;&#123; `_id`: [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; ; `_score`: `number`  &#125;[]&gt;

最も近いベクターを持つドキュメントの Id とスコアを含む Promise

#### 定義場所 \{#defined-in\}

[server/registration.ts:258](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L258)