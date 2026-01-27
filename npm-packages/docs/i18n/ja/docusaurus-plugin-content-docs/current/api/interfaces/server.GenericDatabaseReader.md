---
id: "server.GenericDatabaseReader"
title: "インターフェース: GenericDatabaseReader<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericDatabaseReader

Convex のクエリ関数内でデータベースから読み取るためのインターフェースです。

主なエントリポイントは次の 2 つです:

* [get](server.GenericDatabaseReader.md#get), which fetches a single document
  by its [GenericId](../modules/values.md#genericid).
  * [query](server.GenericDatabaseReader.md#query), which starts building a query.

コード生成を使用している場合は、データモデルに合わせて型付けされた
`convex/_generated/server.d.ts` 内の `DatabaseReader` 型を使用してください。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## 継承階層 \{#hierarchy\}

* `BaseDatabaseReader`&lt;`DataModel`&gt;

  ↳ **`GenericDatabaseReader`**

  ↳↳ [`GenericDatabaseWriter`](server.GenericDatabaseWriter.md)

## プロパティ \{#properties\}

### system \{#system\}

• **system**: `BaseDatabaseReader`&lt;[`SystemDataModel`](server.SystemDataModel.md)&gt;

Convex のクエリ関数内でシステムテーブルから読み取るためのインターフェース

The two entry points are:

* [get](server.GenericDatabaseReader.md#get), which fetches a single document
  by its [GenericId](../modules/values.md#genericid).
  * [query](server.GenericDatabaseReader.md#query), which starts building a query.

#### 定義場所 \{#defined-in\}

[server/database.ts:128](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L128)

## メソッド \{#methods\}

### get \{#get\}

▸ **get**&lt;`TableName`&gt;(`table`, `id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

[GenericId](../modules/values.md#genericid) を指定して、データベースから単一のドキュメントを取得します。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `table` | `TableName` | ドキュメントを取得する対象のテーブル名。 |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`NonUnion`&lt;`TableName`&gt;&gt; | データベースから取得するドキュメントの [GenericId](../modules/values.md#genericid)。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* 指定された [GenericId](../modules/values.md#genericid) に対応するドキュメントの [GenericDocument](../modules/server.md#genericdocument)。そのドキュメントが既に存在しない場合は `null`。

#### 継承元 \{#inherited-from\}

BaseDatabaseReader.get

#### 定義場所 \{#defined-in\}

[server/database.ts:23](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L23)

▸ **get**&lt;`TableName`&gt;(`id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

[GenericId](../modules/values.md#genericid) を指定してデータベースから 1 件のドキュメントを取得します。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | データベースから取得するドキュメントを識別する [GenericId](../modules/values.md#genericid)。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* 指定された [GenericId](../modules/values.md#genericid) に対応するドキュメントの [GenericDocument](../modules/server.md#genericdocument)、またはドキュメントが既に存在しない場合は `null`。

#### 継承元 \{#inherited-from\}

BaseDatabaseReader.get

#### 定義場所 \{#defined-in\}

[server/database.ts:34](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L34)

***

### query \{#query\}

▸ **query**&lt;`TableName`&gt;(`tableName`): [`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

指定したテーブル名に対するクエリを開始します。

クエリは即座には実行されないため、このメソッドを呼び出してクエリを構築・拡張しても、
結果が実際に使用されるまではコストは発生しません。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `tableName` | `TableName` | クエリ対象のテーブル名。 |

#### 戻り値 \{#returns\}

[`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

* クエリの構築を開始するための [QueryInitializer](server.QueryInitializer.md) オブジェクト。

#### 継承元 \{#inherited-from\}

BaseDatabaseReader.query

#### 定義元 \{#defined-in\}

[server/database.ts:47](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L47)

***

### normalizeId \{#normalizeid\}

▸ **normalizeId**&lt;`TableName`&gt;(`tableName`, `id`): `null` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;

指定されたテーブル内の ID について、その ID 文字列形式を返します。ID が別のテーブルのものか、または有効な ID でない場合は null を返します。

ID 文字列形式だけでなく、従来のクラスベース ID 形式の `.toString()` の結果も受け付けます。

これは ID の存在を保証するものではありません（つまり、`db.get(id)` が `null` を返す場合があります）。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### パラメータ \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `tableName` | `TableName` | テーブルの名前。 |
| `id` | `string` | ID を表す文字列。 |

#### 戻り値 \{#returns\}

`null` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;

#### 継承元 \{#inherited-from\}

BaseDatabaseReader.normalizeId

#### 定義元 \{#defined-in\}

[server/database.ts:63](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L63)