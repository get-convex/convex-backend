---
id: "server.GenericDatabaseReaderWithTable"
title: "インターフェース: GenericDatabaseReaderWithTable<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericDatabaseReaderWithTable

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## 継承階層 \{#hierarchy\}

* `BaseDatabaseReaderWithTable`&lt;`DataModel`&gt;

  ↳ **`GenericDatabaseReaderWithTable`**

  ↳↳ [`GenericDatabaseWriterWithTable`](server.GenericDatabaseWriterWithTable.md)

## プロパティ \{#properties\}

### system \{#system\}

• **system**: `BaseDatabaseReaderWithTable`&lt;[`SystemDataModel`](server.SystemDataModel.md)&gt;

Convex のクエリ関数内でシステムテーブルからデータを読み取るためのインターフェースです。

2 つのエントリポイントがあります:

* [get](server.GenericDatabaseReader.md#get): [GenericId](../modules/values.md#genericid) を指定して単一のドキュメントを取得します。
  * [query](server.GenericDatabaseReader.md#query): クエリの構築を開始します。

#### 定義場所 \{#defined-in\}

[server/database.ts:144](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L144)

## メソッド \{#methods\}

### table \{#table\}

▸ **table**&lt;`TableName`&gt;(`tableName`): [`BaseTableReader`](server.BaseTableReader.md)&lt;`DataModel`, `TableName`&gt;

データベースの対象を特定のテーブルに限定します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `tableName` | `TableName` |

#### 戻り値 \{#returns\}

[`BaseTableReader`](server.BaseTableReader.md)&lt;`DataModel`, `TableName`&gt;

#### 継承元 \{#inherited-from\}

BaseDatabaseReaderWithTable.table

#### 定義場所 \{#defined-in\}

[server/database.ts:73](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L73)