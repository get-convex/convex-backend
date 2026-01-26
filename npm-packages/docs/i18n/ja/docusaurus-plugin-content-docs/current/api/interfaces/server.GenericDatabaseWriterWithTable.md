---
id: "server.GenericDatabaseWriterWithTable"
title: "インターフェイス: GenericDatabaseWriterWithTable<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericDatabaseWriterWithTable

Convex のミューテーション関数内で、データベースからの読み取りと書き込みを行うためのインターフェースです。

Convex は、1つのミューテーション内のすべての書き込みがアトミックに
実行されることを保証しているため、部分的な書き込みによってデータが不整合な状態に
なる心配はありません。関数に対して Convex が提供する保証については
[Convex ガイド](https://docs.convex.dev/understanding/convex-fundamentals/functions#atomicity-and-optimistic-concurrency-control)
を参照してください。

コード生成を使用している場合は、データモデル用に型定義された
`convex/_generated/server.d.ts` 内の `DatabaseReader` 型を使用してください。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## 継承階層 \{#hierarchy\}

* [`GenericDatabaseReaderWithTable`](server.GenericDatabaseReaderWithTable.md)&lt;`DataModel`&gt;

  ↳ **`GenericDatabaseWriterWithTable`**

## プロパティ \{#properties\}

### system \{#system\}

• **system**: `BaseDatabaseReaderWithTable`&lt;[`SystemDataModel`](server.SystemDataModel.md)&gt;

Convex のクエリ関数内でシステムテーブルを読み取るためのインターフェースです。

2 つのエントリーポイントがあります:

* [get](server.GenericDatabaseReader.md#get): 単一のドキュメントを
  その [GenericId](../modules/values.md#genericid) によって取得します。
  * [query](server.GenericDatabaseReader.md#query): クエリの構築を開始します。

#### 継承元 \{#inherited-from\}

[GenericDatabaseReaderWithTable](server.GenericDatabaseReaderWithTable.md).[system](server.GenericDatabaseReaderWithTable.md#system)

#### 定義元 \{#defined-in\}

[server/database.ts:144](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L144)

## メソッド \{#methods\}

### table \{#table\}

▸ **table**&lt;`TableName`&gt;(`tableName`): [`BaseTableWriter`](server.BaseTableWriter.md)&lt;`DataModel`, `TableName`&gt;

データベース操作の対象を特定のテーブルに限定します。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### パラメータ \{#parameters\}

| パラメータ名 | 型 |
| :------ | :------ |
| `tableName` | `TableName` |

#### 戻り値 \{#returns\}

[`BaseTableWriter`](server.BaseTableWriter.md)&lt;`DataModel`, `TableName`&gt;

#### オーバーライド \{#overrides\}

[GenericDatabaseReaderWithTable](server.GenericDatabaseReaderWithTable.md).[table](server.GenericDatabaseReaderWithTable.md#table)

#### 定義場所 \{#defined-in\}

[server/database.ts:274](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L274)