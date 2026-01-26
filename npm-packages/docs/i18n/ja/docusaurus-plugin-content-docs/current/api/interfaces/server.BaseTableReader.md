---
id: "server.BaseTableReader"
title: "インターフェース: BaseTableReader<DataModel, TableName>"
custom_edit_url: null
---

[server](../modules/server.md).BaseTableReader

## 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](../modules/server.md#genericdatamodel) |
| `TableName` | extends [`TableNamesInDataModel`](../modules/server.md#tablenamesindatamodel)&lt;`DataModel`&gt; |

## 継承階層 \{#hierarchy\}

* **`BaseTableReader`**

  ↳ [`BaseTableWriter`](server.BaseTableWriter.md)

## メソッド \{#methods\}

### get \{#get\}

▸ **get**(`id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

[GenericId](../modules/values.md#genericid) を指定して、テーブルから 1 件のドキュメントを取得します。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | データベースから取得するドキュメントを指す [GenericId](../modules/values.md#genericid)。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* 指定された [GenericId](../modules/values.md#genericid) に対応するドキュメントの [GenericDocument](../modules/server.md#genericdocument)。もはや存在しない場合は `null`。

#### 定義場所 \{#defined-in\}

[server/database.ts:88](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L88)

***

### query \{#query\}

▸ **query**(): [`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

このテーブルに対するクエリを開始します。

クエリはすぐには実行されないため、このメソッドを呼び出してクエリを拡張・組み立てても、
実際に結果が利用されるまではオーバーヘッドは発生しません。

#### Returns \{#returns\}

[`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

* クエリの構築を開始するための [QueryInitializer](server.QueryInitializer.md) オブジェクト。

#### 定義元 \{#defined-in\}

[server/database.ts:100](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L100)