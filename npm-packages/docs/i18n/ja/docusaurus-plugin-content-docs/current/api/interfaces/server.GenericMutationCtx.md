---
id: "server.GenericMutationCtx"
title: "インターフェース: GenericMutationCtx<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericMutationCtx

Convex のミューテーション関数内で使用するサービス群です。

ミューテーションコンテキストは、サーバー上で実行されるすべての Convex ミューテーション
関数に対して、最初の引数として渡されます。

コード生成を使用している場合は、データモデルに対応した型が付けられている
`convex/_generated/server.d.ts` 内の `MutationCtx` 型を使用してください。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## プロパティ \{#properties\}

### db \{#db\}

• **db**: [`GenericDatabaseWriter`](server.GenericDatabaseWriter.md)&lt;`DataModel`&gt;

データベース内のデータを読み書きするためのユーティリティです。

#### 定義場所 \{#defined-in\}

[server/registration.ts:50](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L50)

***

### auth \{#auth\}

• **auth**: [`Auth`](server.Auth.md)

現在の認証済みユーザーに関する情報。

#### 定義場所 \{#defined-in\}

[server/registration.ts:55](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L55)

***

### storage \{#storage\}

• **storage**: [`StorageWriter`](server.StorageWriter.md)

ストレージ内のファイルを読み書きするためのユーティリティです。

#### 定義場所 \{#defined-in\}

[server/registration.ts:60](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L60)

***

### scheduler \{#scheduler\}

• **scheduler**: [`Scheduler`](server.Scheduler.md)

将来に実行する Convex 関数をスケジュールするためのユーティリティです。

#### 定義場所 \{#defined-in\}

[server/registration.ts:65](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L65)

***

### runQuery \{#runquery\}

• **runQuery**: &lt;Query&gt;(`query`: `Query`, ...`args`: [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt;) =&gt; `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

#### 型宣言 \{#type-declaration\}

▸ &lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

同じトランザクション内でクエリ関数を呼び出します。

注意: 多くの場合、このメソッドを使わずにクエリ関数を直接呼び出せます。
`runQuery` には、引数と戻り値の検証を行うオーバーヘッドに加えて、
新しい分離された JS コンテキストを作成するコストも発生します。

##### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`, `"public"` | `"internal"`&gt; |

##### パラメータ \{#parameters\}

| パラメータ名 | 型 |
| :------ | :------ |
| `query` | `Query` |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; |

##### 戻り値 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

#### 定義元 \{#defined-in\}

[server/registration.ts:74](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L74)

***

### runMutation \{#runmutation\}

• **runMutation**: &lt;Mutation&gt;(`mutation`: `Mutation`, ...`args`: [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Mutation`&gt;) =&gt; `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

#### 型宣言 \{#type-declaration\}

▸ &lt;`Mutation`&gt;(`mutation`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

同一トランザクション内でミューテーション関数を呼び出します。

注意: 多くの場合、`runMutation` を使わずにミューテーションの関数を直接呼び出すことができます。
`runMutation` には、引数と戻り値の検証を実行するためのオーバーヘッドと、
新しい分離された JS コンテキストを作成するためのオーバーヘッドがあります。

ミューテーションはサブトランザクション内で実行されるため、ミューテーションがエラーをスローした場合、
その書き込みはすべてロールバックされます。さらに、成功したミューテーションの
書き込みは、そのトランザクション内の他の書き込みとの間でシリアライズ可能であることが保証されます。

##### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`, `"public"` | `"internal"`&gt; |

##### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `mutation` | `Mutation` |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Mutation`&gt; |

##### 戻り値 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

#### 定義元 \{#defined-in\}

[server/registration.ts:90](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L90)