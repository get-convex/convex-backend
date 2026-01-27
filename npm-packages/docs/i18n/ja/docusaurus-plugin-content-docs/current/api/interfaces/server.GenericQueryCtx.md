---
id: "server.GenericQueryCtx"
title: "インターフェース: GenericQueryCtx<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericQueryCtx

Convex のクエリ関数内で使用するためのサービス群です。

クエリコンテキストは、サーバー上で実行されるすべての Convex クエリ関数に、最初の引数として渡されます。

すべてのサービスが読み取り専用である点で、これは MutationCtx とは異なります。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## プロパティ \{#properties\}

### db \{#db\}

• **db**: [`GenericDatabaseReader`](server.GenericDatabaseReader.md)&lt;`DataModel`&gt;

データベースからデータを読み取るためのユーティリティです。

#### 定義場所 \{#defined-in\}

[server/registration.ts:130](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L130)

***

### auth \{#auth\}

• **auth**: [`Auth`](server.Auth.md)

現在認証されているユーザーに関する情報です。

#### 定義場所 \{#defined-in\}

[server/registration.ts:135](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L135)

***

### storage \{#storage\}

• **storage**: [`StorageReader`](server.StorageReader.md)

ストレージ内のファイルを読み込むためのユーティリティです。

#### 定義元 \{#defined-in\}

[server/registration.ts:140](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L140)

***

### runQuery \{#runquery\}

• **runQuery**: &lt;Query&gt;(`query`: `Query`, ...`args`: [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt;) =&gt; `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

#### 型宣言 \{#type-declaration\}

▸ &lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

同一トランザクション内でクエリ関数を呼び出します。

注意: 多くの場合、このメソッドを使わずにクエリ関数を直接呼び出せます。
`runQuery` を使用すると、引数および戻り値の検証と、
新しい分離された JS コンテキストの作成のためにオーバーヘッドが発生します。

##### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`, `"public"` | `"internal"`&gt; |

##### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `query` | `Query` |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; |

##### 戻り値 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

#### 定義元 \{#defined-in\}

[server/registration.ts:149](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L149)