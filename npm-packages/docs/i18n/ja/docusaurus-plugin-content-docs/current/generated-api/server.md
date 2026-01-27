---
title: "server.js"
sidebar_position: 4
description:
  "Convex のクエリ、ミューテーション、アクションを実装するために生成されるユーティリティ"
---

<Admonition type="caution" title="このコードは生成されています">
  これらの export は `convex` パッケージから直接利用することはできません。

  代わりに `npx convex dev` を実行して、`convex/_generated/server.js`
  および `convex/_generated/server.d.ts` を作成する必要があります。
</Admonition>

サーバー側の Convex クエリおよびミューテーション関数を実装するための生成ユーティリティです。

## 関数 \{#functions\}

### query \{#query\}

▸ **query**(`func`): [`RegisteredQuery`](/api/modules/server#registeredquery)

この Convex アプリのパブリック API にクエリを定義します。

この関数は Convex データベースを読み取り、
クライアントからアクセス可能になります。

これは [`queryGeneric`](/api/modules/server#querygeneric) のエイリアスであり、
このアプリのデータモデル向けに型付けされています。

#### 引数 \{#parameters\}

| 名前   | 説明                                                                                          |
| :----- | :-------------------------------------------------------------------------------------------- |
| `func` | クエリ関数。先頭の引数として [QueryCtx](server.md#queryctx) を受け取ります。 |

#### 戻り値 \{#returns\}

[`RegisteredQuery`](/api/modules/server#registeredquery)

ラップされたクエリ。これを `export` して名前を付け、アクセスできるようにします。

***

### internalQuery \{#internalquery\}

▸ **internalQuery**(`func`):
[`RegisteredQuery`](/api/modules/server#registeredquery)

他の Convex 関数からのみ呼び出すことができるクエリを定義します（クライアントからは
呼び出せません）。

この関数は Convex データベースからの読み取りが許可されます。クライアントからは
アクセスできません。

これは
[`internalQueryGeneric`](/api/modules/server#internalquerygeneric) のエイリアスであり、
アプリのデータモデル向けに型付けされています。

#### パラメータ \{#parameters\}

| 名前   | 説明                                                                                           |
| :----- | :--------------------------------------------------------------------------------------------- |
| `func` | クエリ関数。最初の引数として [QueryCtx](server.md#queryctx) を受け取ります。 |

#### 戻り値 \{#returns\}

[`RegisteredQuery`](/api/modules/server#registeredquery)

ラップされたクエリ。これを `export` して名前を付け、アクセスできるようにします。

***

### ミューテーション \{#mutation\}

▸ **mutation**(`func`):
[`RegisteredMutation`](/api/modules/server#registeredmutation)

この Convex アプリのパブリック API におけるミューテーションを定義します。

この関数は Convex データベースを変更でき、
クライアントからアクセス可能です。

これは、このアプリのデータモデル向けに型付けされた [`mutationGeneric`](/api/modules/server#mutationgeneric)
のエイリアスです。

#### パラメータ \{#parameters\}

| Name   | Description                                                                                 |
| :----- | :------------------------------------------------------------------------------------------ |
| `func` | ミューテーション関数。最初の引数として [MutationCtx](#mutationctx) を受け取ります。 |

#### Returns \{#returns\}

[`RegisteredMutation`](/api/modules/server#registeredmutation)

ラップされたミューテーションです。これを `export` として公開し、名前を付けて利用できるようにします。

***

### internalMutation \{#internalmutation\}

▸ **internalMutation**(`func`):
[`RegisteredMutation`](/api/modules/server#registeredmutation)

他の Convex 関数からのみアクセス可能なミューテーションを定義します（クライアントからはアクセスできません）。

この関数は Convex データベースへの読み取りと書き込みを行えますが、クライアントからはアクセスできません。

これは、あなたのアプリのデータモデル用に型付けされた
[`internalMutationGeneric`](/api/modules/server#internalmutationgeneric)
のエイリアスです。

#### パラメーター \{#parameters\}

| 名前   | 説明                                                                                                      |
| :----- | :-------------------------------------------------------------------------------------------------------- |
| `func` | ミューテーション関数。最初の引数として [MutationCtx](server.md#mutationctx) を受け取ります。 |

#### 戻り値 \{#returns\}

[`RegisteredMutation`](/api/modules/server#registeredmutation)

ラップされたミューテーション。これを `export` して名前を付け、
アクセスできるようにします。

***

### action \{#action\}

▸ **action**(`func`): [`RegisteredAction`](/api/modules/server#registeredaction)

この Convex アプリのパブリック API でアクションを定義します。

アクションは、外部サービスの呼び出しのような副作用を伴うコードや
非決定的なコードも含め、任意の JavaScript コードを実行できる関数です。
Convex の JavaScript 環境、または `"use node"` ディレクティブを使って Node.js 上で実行できます。
[`ActionCtx`](#actionctx) を使ってクエリやミューテーションを呼び出すことで、
データベースと間接的にやり取りできます。

これは、アプリのデータモデル向けに型付けされた
[`actionGeneric`](/api/modules/server#actiongeneric) のエイリアスです。

#### パラメータ \{#parameters\}

| Name   | Description                                                                                      |
| :----- | :----------------------------------------------------------------------------------------------- |
| `func` | アクション関数で、最初の引数として [ActionCtx](#actionctx) を受け取ります。 |

#### 戻り値 \{#returns\}

[`RegisteredAction`](/api/modules/server#registeredaction)

ラップされた関数。これを `export` して名前を付け、アクセス可能にします。

***

### internalAction \{#internalaction\}

▸ **internalAction**(`func`):
[`RegisteredAction`](/api/modules/server#registeredaction)

他の Convex 関数からのみ呼び出すことができ（クライアントからは呼び出せない）アクションを定義します。

これは
[`internalActionGeneric`](/api/modules/server#internalactiongeneric)
のエイリアスであり、アプリのデータモデルに合わせて型付けされています。

#### パラメータ \{#parameters\}

| Name   | Description                                                                                 |
| :----- | :------------------------------------------------------------------------------------------ |
| `func` | アクション関数です。最初の引数として [ActionCtx](server.md#actionctx) を受け取ります。 |

#### 戻り値 \{#returns\}

[`RegisteredAction`](/api/modules/server#registeredaction)

ラップされたアクションです。これを `export` して名前を付け、アクセスできるようにします。

***

### httpAction \{#httpaction\}

▸
**httpAction**(`func: (ctx: ActionCtx, request: Request) => Promise<Response>`):
[`PublicHttpAction`](/api/modules/server#publichttpaction)

#### Parameters \{#parameters\}

| Name   | Type                                                      | Description                                                                                                                                                                                         |
| :----- | :-------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `func` | `(ctx: ActionCtx, request: Request) => Promise<Response>` | 関数です。この関数は最初の引数として [`ActionCtx`](/api/modules/server#actionctx)、2 番目の引数として [`Request`](https://developer.mozilla.org/en-US/docs/Web/API/Request) を受け取ります。 |

#### 戻り値 \{#returns\}

[`PublicHttpAction`](/api/modules/server#publichttpaction)

ラップされた関数。`convex/http.js` からこの関数をインポートし、ルーティングに組み込んでください。

## 型 \{#types\}

### QueryCtx \{#queryctx\}

Ƭ **QueryCtx**: `Object`

Convex のクエリ関数内で使用するためのサービスの集合。

クエリコンテキストは、サーバー上で実行されるすべての Convex クエリ関数に対して、
最初の引数として渡されます。

これは [MutationCtx](#mutationctx) とは異なり、すべてのサービスが
読み取り専用です。

これは、アプリのデータモデル向けに型付けされた [`GenericQueryCtx`](/api/interfaces/server.GenericQueryCtx)
のエイリアスです。

#### 型定義 \{#type-declaration\}

| 名前     | 型                                                          |
| :-------- | :--------------------------------------------------------- |
| `db`      | [`DatabaseReader`](#databasereader)                        |
| `auth`    | [`Auth`](/api/interfaces/server.Auth.md)                   |
| `storage` | [`StorageReader`](/api/interfaces/server.StorageReader.md) |

***

### MutationCtx \{#mutationctx\}

Ƭ **MutationCtx**: `Object`

Convex のミューテーション関数内で使用するサービス群です。

ミューテーションコンテキストは、サーバー上で実行されるすべての Convex ミューテーション
関数に対して、最初の引数として渡されます。

これは
あなたのアプリのデータモデル用に型付けされた
[`GenericMutationCtx`](/api/interfaces/server.GenericMutationCtx) のエイリアスです。

#### 型定義 \{#type-declaration\}

| 名前       | 型                                                         |
| :---------- | :--------------------------------------------------------- |
| `db`        | [`DatabaseWriter`](#databasewriter)                        |
| `auth`      | [`Auth`](/api/interfaces/server.Auth.md)                   |
| `storage`   | [`StorageWriter`](/api/interfaces/server.StorageWriter.md) |
| `scheduler` | [`Scheduler`](/api/interfaces/server.Scheduler.md)         |

***

### ActionCtx \{#actionctx\}

Ƭ **ActionCtx**: `Object`

Convex のアクション関数内で使用するための一連のサービスです。

アクションコンテキストは、サーバー上で実行されるすべての Convex アクション関数に
最初の引数として渡されます。

これは、アプリのデータモデル用に型付けされた [`ActionCtx`](/api/modules/server#actionctx) のエイリアスです。

#### 型定義 \{#type-declaration\}

| 名前           | 型                                                                                                                                                                          |
| :------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `runQuery`     | (`name`: `string`, `args`?: `Record<string, Value>`) =&gt; `Promise<Value>`                                                                                                     |
| `runMutation`  | (`name`: `string`, `args`?: `Record<string, Value>`) =&gt; `Promise<Value>`                                                                                                     |
| `runAction`    | (`name`: `string`, `args`?: `Record<string, Value>`) =&gt; `Promise<Value>`                                                                                                     |
| `auth`         | [`Auth`](/api/interfaces/server.Auth.md)                                                                                                                                     |
| `scheduler`    | [`Scheduler`](/api/interfaces/server.Scheduler.md)                                                                                                                           |
| `storage`      | [`StorageActionWriter`](/api/interfaces/server.StorageActionWriter.md)                                                                                                       |
| `vectorSearch` | (`tableName`: `string`, `indexName`: `string`, `query`: [`VectorSearchQuery`](/api/interfaces/server.VectorSearchQuery.md)) =&gt; `Promise<Array<{ _id: Id, _score: number }>>` |

***

### DatabaseReader \{#databasereader\}

Convex のクエリ関数内でデータベースから読み出すためのインターフェースです。

これは、あなたのアプリのデータモデルに合わせて型付けされた
[`GenericDatabaseReader`](/api/interfaces/server.GenericDatabaseReader)
のエイリアスです。

***

### DatabaseWriter \{#databasewriter\}

Convex のミューテーション関数内でデータベースの読み取りと書き込みを行うためのインターフェースです。

これは、アプリのデータモデルに対して型付けされた
[`GenericDatabaseWriter`](/api/interfaces/server.GenericDatabaseWriter) のエイリアスです。