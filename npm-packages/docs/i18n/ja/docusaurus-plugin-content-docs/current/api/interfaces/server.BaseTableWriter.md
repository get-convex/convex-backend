---
id: "server.BaseTableWriter"
title: "インターフェース: BaseTableWriter<DataModel, TableName>"
custom_edit_url: null
---

[server](../modules/server.md).BaseTableWriter

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DataModel` | [`GenericDataModel`](../modules/server.md#genericdatamodel) を拡張 |
| `TableName` | [`TableNamesInDataModel`](../modules/server.md#tablenamesindatamodel)&lt;`DataModel`&gt; を拡張 |

## 継承階層 \{#hierarchy\}

* [`BaseTableReader`](server.BaseTableReader.md)&lt;`DataModel`, `TableName`&gt;

  ↳ **`BaseTableWriter`**

## メソッド \{#methods\}

### get \{#get\}

▸ **get**(`id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

[GenericId](../modules/values.md#genericid) で指定されたテーブルのドキュメントを 1 件取得します。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | データベースから取得する対象ドキュメントの [GenericId](../modules/values.md#genericid)。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* 指定された [GenericId](../modules/values.md#genericid) のドキュメントに対応する [GenericDocument](../modules/server.md#genericdocument)、またはドキュメントがすでに存在しない場合は `null`。

#### 継承元 \{#inherited-from\}

[BaseTableReader](server.BaseTableReader.md).[get](server.BaseTableReader.md#get)

#### 定義場所 \{#defined-in\}

[server/database.ts:88](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L88)

***

### query \{#query\}

▸ **query**(): [`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

テーブルに対するクエリを開始します。

クエリは直ちには実行されないため、このメソッドを呼び出してクエリを組み立てても、結果が実際に利用されるまではコストは発生しません。

#### 戻り値 \{#returns\}

[`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

* クエリを構築し始めるための [QueryInitializer](server.QueryInitializer.md) オブジェクト。

#### 継承元 \{#inherited-from\}

[BaseTableReader](server.BaseTableReader.md).[query](server.BaseTableReader.md#query)

#### 定義場所 \{#defined-in\}

[server/database.ts:100](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L100)

***

### insert \{#insert\}

▸ **insert**(`value`): `Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;&gt;

新しいドキュメントをテーブルに挿入します。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `value` | [`WithoutSystemFields`](../modules/server.md#withoutsystemfields)&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | 指定されたテーブルに挿入するための[値](../modules/values.md#value)。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;&gt;

* 挿入された新しいドキュメントの [GenericId](../modules/values.md#genericid)。

#### 定義場所 \{#defined-in\}

[server/database.ts:289](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L289)

***

### patch \{#patch\}

▸ **patch**(`id`, `value`): `Promise`&lt;`void`&gt;

既存のドキュメントを、指定された部分ドキュメントとシャロー マージして更新します。

新しいフィールドは追加されます。既存のフィールドは上書きされます。`undefined` が設定されたフィールドは削除されます。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | パッチを適用するドキュメントの [GenericId](../modules/values.md#genericid)。 |
| `value` | `PatchValue`&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | 指定されたドキュメントにマージするための部分的な [GenericDocument](../modules/server.md#genericdocument)。この新しい値が `_id` のようなシステムフィールドを指定する場合、それらはドキュメントに既に存在するフィールド値と一致していなければなりません。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定義場所 \{#defined-in\}

[server/database.ts:304](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L304)

***

### replace \{#replace\}

▸ **replace**(`id`, `value`): `Promise`&lt;`void`&gt;

既存のドキュメントの値を新しい値で置き換え、古い値を上書きします。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | 置換する対象ドキュメントの [GenericId](../modules/values.md#genericid)。 |
| `value` | [`WithOptionalSystemFields`](../modules/server.md#withoptionalsystemfields)&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | ドキュメント用の新しい [GenericDocument](../modules/server.md#genericdocument)。この値ではシステムフィールドを省略でき、データベース側で自動的に補完されます。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定義場所 \{#defined-in\}

[server/database.ts:316](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L316)

***

### delete \{#delete\}

▸ **delete**(`id`): `Promise`&lt;`void`&gt;

既存のドキュメントを削除します。

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | 削除するドキュメントの [GenericId](../modules/values.md#genericid)。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定義場所 \{#defined-in\}

[server/database.ts:326](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L326)