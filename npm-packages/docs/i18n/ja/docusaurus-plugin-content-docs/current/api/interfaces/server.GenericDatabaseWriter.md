---
id: "server.GenericDatabaseWriter"
title: "インターフェース: GenericDatabaseWriter<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericDatabaseWriter

Convex のミューテーション関数内でデータベースの読み書きを行うためのインターフェースです。

Convex は、1 回のミューテーション内のすべての書き込みがアトミックに実行されることを保証しているため、
部分的な書き込みによってデータが不整合な状態のまま残ってしまう心配はありません。
Convex が関数に対して提供する保証については、
[Convex ガイド](https://docs.convex.dev/understanding/convex-fundamentals/functions#atomicity-and-optimistic-concurrency-control)を参照してください。

コード生成を使用している場合は、データモデルに対して型付けされた
`convex/_generated/server.d.ts` 内の `DatabaseReader` 型を使用してください。

## 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## 継承階層 \{#hierarchy\}

* [`GenericDatabaseReader`](server.GenericDatabaseReader.md)&lt;`DataModel`&gt;

  ↳ **`GenericDatabaseWriter`**

## プロパティ \{#properties\}

### system \{#system\}

• **system**: `BaseDatabaseReader`&lt;[`SystemDataModel`](server.SystemDataModel.md)&gt;

Convex のクエリ関数内でシステムテーブルから読み出すためのインターフェース

エントリポイントは次の2つです:

* [get](server.GenericDatabaseReader.md#get), which fetches a single document
  by its [GenericId](../modules/values.md#genericid).
  * [query](server.GenericDatabaseReader.md#query), which starts building a query.

#### 継承元 \{#inherited-from\}

[GenericDatabaseReader](server.GenericDatabaseReader.md).[system](server.GenericDatabaseReader.md#system)

#### 定義元 \{#defined-in\}

[server/database.ts:128](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L128)

## メソッド \{#methods\}

### get \{#get\}

▸ **get**&lt;`TableName`&gt;(`table`, `id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

データベースから、[GenericId](../modules/values.md#genericid) で指定された 1 件のドキュメントを取得します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### パラメータ \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `table` | `TableName` | ドキュメントを取得するテーブルの名前。 |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`NonUnion`&lt;`TableName`&gt;&gt; | データベースから取得するドキュメントの [GenericId](../modules/values.md#genericid)。 |

#### Returns \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* 指定された [GenericId](../modules/values.md#genericid) のドキュメントに対応する [GenericDocument](../modules/server.md#genericdocument)、もはや存在しない場合は `null`。

#### 継承元 \{#inherited-from\}

[GenericDatabaseReader](server.GenericDatabaseReader.md).[get](server.GenericDatabaseReader.md#get)

#### 定義場所 \{#defined-in\}

[server/database.ts:23](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L23)

▸ **get**&lt;`TableName`&gt;(`id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

[GenericId](../modules/values.md#genericid) によってデータベースから単一のドキュメントを取得します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | データベースから取得する対象ドキュメントの [GenericId](../modules/values.md#genericid)。 |

#### 返り値 \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* 指定された [GenericId](../modules/values.md#genericid) に対応するドキュメントの [GenericDocument](../modules/server.md#genericdocument)、もしくは、ドキュメントがすでに存在しない場合は `null`。

#### 継承元 \{#inherited-from\}

[GenericDatabaseReader](server.GenericDatabaseReader.md).[get](server.GenericDatabaseReader.md#get)

#### 定義場所 \{#defined-in\}

[server/database.ts:34](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L34)

***

### query \{#query\}

▸ **query**&lt;`TableName`&gt;(`tableName`): [`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

指定されたテーブル名に対してクエリを開始します。

クエリはすぐには実行されないため、このメソッドを呼び出してクエリを拡張しても、
結果が実際に使用されるまではオーバーヘッドは発生しません。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `tableName` | `TableName` | クエリ対象のテーブル名。 |

#### 戻り値 \{#returns\}

[`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

* クエリの構築を開始するために使用する [QueryInitializer](server.QueryInitializer.md) オブジェクト。

#### 継承元 \{#inherited-from\}

[GenericDatabaseReader](server.GenericDatabaseReader.md).[query](server.GenericDatabaseReader.md#query)

#### 定義元 \{#defined-in\}

[server/database.ts:47](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L47)

***

### normalizeId \{#normalizeid\}

▸ **normalizeId**&lt;`TableName`&gt;(`tableName`, `id`): `null` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;

指定されたテーブル内の ID に対して、その文字列表現（文字列 ID 形式）を返します。\
ID が別のテーブルのもの、または無効な ID の場合は null を返します。

これは、文字列 ID 形式だけでなく、レガシーなクラスベース ID 形式の
`.toString()` による文字列表現も受け付けます。

これは ID の存在を保証するものではありません（つまり、`db.get(id)` が `null` を返す場合があります）。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### パラメーター \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `tableName` | `TableName` | テーブルの名前。 |
| `id` | `string` | ID を表す文字列。 |

#### 戻り値 \{#returns\}

`null` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;

#### 継承元 \{#inherited-from\}

[GenericDatabaseReader](server.GenericDatabaseReader.md).[normalizeId](server.GenericDatabaseReader.md#normalizeid)

#### 定義場所 \{#defined-in\}

[server/database.ts:63](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L63)

***

### insert \{#insert\}

▸ **insert**&lt;`TableName`&gt;(`table`, `value`): `Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;&gt;

新しいドキュメントをテーブルに追加します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `table` | `TableName` | 新しいドキュメントを挿入する対象テーブルの名前。 |
| `value` | [`WithoutSystemFields`](../modules/server.md#withoutsystemfields)&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | 指定されたテーブルに挿入する[値](../modules/values.md#value)。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;&gt;

* 新規ドキュメントの [GenericId](../modules/values.md#genericid)。

#### 定義場所 \{#defined-in\}

[server/database.ts:170](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L170)

***

### patch \{#patch\}

▸ **patch**&lt;`TableName`&gt;(`table`, `id`, `value`): `Promise`&lt;`void`&gt;

既存のドキュメントに対して、指定された部分ドキュメントを浅くマージして更新します。

新しいフィールドは追加されます。既存のフィールドは上書きされます。`undefined` が設定されたフィールドは削除されます。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `table` | `TableName` | ドキュメントが属しているテーブルの名前。 |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`NonUnion`&lt;`TableName`&gt;&gt; | パッチを適用するドキュメントの [GenericId](../modules/values.md#genericid)。 |
| `value` | `PatchValue`&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | 指定されたドキュメントにマージする部分的な [GenericDocument](../modules/server.md#genericdocument)。この新しい値で `_id` のようなシステムフィールドを指定する場合、それらはそのドキュメントに既に存在するフィールド値と一致している必要があります。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定義場所 \{#defined-in\}

[server/database.ts:187](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L187)

▸ **patch**&lt;`TableName`&gt;(`id`, `value`): `Promise`&lt;`void`&gt;

既存のドキュメントにパッチを適用し、指定された部分ドキュメントとシャロー マージします。

新しいフィールドは追加されます。既存のフィールドは上書きされます。`undefined` に設定されたフィールドは削除されます。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | パッチを適用するドキュメントの [GenericId](../modules/values.md#genericid)。 |
| `value` | `PatchValue`&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | 指定されたドキュメントにマージする部分的な [GenericDocument](../modules/server.md#genericdocument)。この新しい値で `_id` などのシステムフィールドを指定する場合、その値はドキュメントの既存のフィールド値と一致している必要があります。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定義元 \{#defined-in\}

[server/database.ts:204](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L204)

***

### replace \{#replace\}

▸ **replace**&lt;`TableName`&gt;(`table`, `id`, `value`): `Promise`&lt;`void`&gt;

既存のドキュメントの値を新しい値で置き換え、古い値を上書きします。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `table` | `TableName` | ドキュメントが属するテーブル名。 |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`NonUnion`&lt;`TableName`&gt;&gt; | 置き換えるドキュメントの [GenericId](../modules/values.md#genericid)。 |
| `value` | [`WithOptionalSystemFields`](../modules/server.md#withoptionalsystemfields)&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | ドキュメント用の新しい [GenericDocument](../modules/server.md#genericdocument)。この値ではシステムフィールドを省略でき、データベースが自動で補完します。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定義場所 \{#defined-in\}

[server/database.ts:217](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L217)

▸ **replace**&lt;`TableName`&gt;(`id`, `value`): `Promise`&lt;`void`&gt;

既存のドキュメントの値を置き換えて、元の値を上書きします。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | 置換するドキュメントの [GenericId](../modules/values.md#genericid)。 |
| `value` | [`WithOptionalSystemFields`](../modules/server.md#withoptionalsystemfields)&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | 対象ドキュメントの新しい [GenericDocument](../modules/server.md#genericdocument)。この値ではシステムフィールドを省略でき、データベースが自動的に補完します。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定義場所 \{#defined-in\}

[server/database.ts:230](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L230)

***

### delete \{#delete\}

▸ **delete**&lt;`TableName`&gt;(`table`, `id`): `Promise`&lt;`void`&gt;

既存のドキュメントを削除します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `table` | `TableName` | ドキュメントが属するテーブルの名前。 |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`NonUnion`&lt;`TableName`&gt;&gt; | 削除対象のドキュメントの [GenericId](../modules/values.md#genericid)。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定義元 \{#defined-in\}

[server/database.ts:241](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L241)

▸ **delete**(`id`): `Promise`&lt;`void`&gt;

既存のドキュメントを削除します。

#### パラメーター \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;[`TableNamesInDataModel`](../modules/server.md#tablenamesindatamodel)&lt;`DataModel`&gt;&gt; | 削除対象のドキュメントの [GenericId](../modules/values.md#genericid)。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定義元 \{#defined-in\}

[server/database.ts:251](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L251)