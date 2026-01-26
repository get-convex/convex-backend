---
id: "server.GenericDatabaseReader"
title: "接口：GenericDatabaseReader<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericDatabaseReader

用于在 Convex 查询函数中从数据库读取数据的接口。

有两个入口：

* [get](server.GenericDatabaseReader.md#get)，按其 [GenericId](../modules/values.md#genericid)
  获取单个文档。
  * [query](server.GenericDatabaseReader.md#query)，用于开始构建查询。

如果你在使用代码生成，请使用 `convex/_generated/server.d.ts` 中的 `DatabaseReader` 类型，
它会根据你的数据模型进行类型定义。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## 继承关系 \{#hierarchy\}

* `BaseDatabaseReader`&lt;`DataModel`&gt;

  ↳ **`GenericDatabaseReader`**

  ↳↳ [`GenericDatabaseWriter`](server.GenericDatabaseWriter.md)

## 属性 \{#properties\}

### system \{#system\}

• **system**: `BaseDatabaseReader`&lt;[`SystemDataModel`](server.SystemDataModel.md)&gt;

在 Convex 查询函数中用于读取系统表的接口

The two entry points are:

* [get](server.GenericDatabaseReader.md#get)，按其 [GenericId](../modules/values.md#genericid) 获取单个文档。
  * [query](server.GenericDatabaseReader.md#query)，用于开始构建查询。

#### 定义于 \{#defined-in\}

[server/database.ts:128](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L128)

## 方法 \{#methods\}

### get \{#get\}

▸ **get**&lt;`TableName`&gt;(`table`, `id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

通过其 [GenericId](../modules/values.md#genericid) 从数据库中获取一条文档。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `table` | `TableName` | 要获取文档所在的表名。 |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`NonUnion`&lt;`TableName`&gt;&gt; | 要从数据库中获取的该文档的 [GenericId](../modules/values.md#genericid)。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* 给定 [GenericId](../modules/values.md#genericid) 对应文档的 [GenericDocument](../modules/server.md#genericdocument)，如果该文档已不存在，则返回 `null`。

#### 继承自 \{#inherited-from\}

BaseDatabaseReader.get

#### 定义于 \{#defined-in\}

[server/database.ts:23](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L23)

▸ **get**&lt;`TableName`&gt;(`id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

通过其 [GenericId](../modules/values.md#genericid) 从数据库中获取单个文档。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | 要从数据库中获取的文档的 [GenericId](../modules/values.md#genericid)。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* 给定 [GenericId](../modules/values.md#genericid) 对应文档的 [GenericDocument](../modules/server.md#genericdocument)，如果该文档已不存在，则返回 `null`。

#### 继承自 \{#inherited-from\}

BaseDatabaseReader.get

#### 定义于 \{#defined-in\}

[server/database.ts:34](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L34)

***

### query \{#query\}

▸ **query**&lt;`TableName`&gt;(`tableName`): [`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

针对给定的表名开始一个查询。

查询不会立即执行，因此在结果实际被使用之前，调用此方法并在其返回的查询基础上继续扩展是没有开销的。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `tableName` | `TableName` | 要查询的表名。 |

#### 返回值 \{#returns\}

[`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

* 一个 [QueryInitializer](server.QueryInitializer.md) 对象，用于开始构建查询。

#### 继承自 \{#inherited-from\}

BaseDatabaseReader.query

#### 定义于 \{#defined-in\}

[server/database.ts:47](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L47)

***

### normalizeId \{#normalizeid\}

▸ **normalizeId**&lt;`TableName`&gt;(`tableName`, `id`): `null` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;

返回给定数据表中某个 ID 的字符串形式，如果该 ID 来自不同的数据表或不是有效 ID，则返回 `null`。

此函数既接受字符串形式的 ID，也接受旧的基于类的 ID 格式的 `.toString()` 表示。

这并不保证该 ID 在数据库中实际存在（即 `db.get(id)` 可能返回 `null`）。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `tableName` | `TableName` | 表名。 |
| `id` | `string` | Id 字符串。 |

#### 返回值 \{#returns\}

`null` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;

#### 继承自 \{#inherited-from\}

BaseDatabaseReader.normalizeId

#### 定义于 \{#defined-in\}

[server/database.ts:63](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L63)