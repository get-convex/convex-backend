---
id: "server.GenericDatabaseReaderWithTable"
title: "接口：GenericDatabaseReaderWithTable<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericDatabaseReaderWithTable

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | 扩展自 [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## 层次结构 \{#hierarchy\}

* `BaseDatabaseReaderWithTable`&lt;`DataModel`&gt;

  ↳ **`GenericDatabaseReaderWithTable`**

  ↳↳ [`GenericDatabaseWriterWithTable`](server.GenericDatabaseWriterWithTable.md)

## 属性 \{#properties\}

### system \{#system\}

• **system**: `BaseDatabaseReaderWithTable`&lt;[`SystemDataModel`](server.SystemDataModel.md)&gt;

用于在 Convex 查询函数中读取系统表的接口

有两个入口点：

* [get](server.GenericDatabaseReader.md#get)，按其 [GenericId](../modules/values.md#genericid) 获取单个文档
  * [query](server.GenericDatabaseReader.md#query)，用于开始构建查询。

#### 定义于 \{#defined-in\}

[server/database.ts:144](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L144)

## 方法 \{#methods\}

### table \{#table\}

▸ **table**&lt;`TableName`&gt;(`tableName`): [`BaseTableReader`](server.BaseTableReader.md)&lt;`DataModel`, `TableName`&gt;

将数据库操作限定在某个特定表上。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 参数 \{#parameters\}

| 参数名 | 类型 |
| :------ | :------ |
| `tableName` | `TableName` |

#### 返回值 \{#returns\}

[`BaseTableReader`](server.BaseTableReader.md)&lt;`DataModel`, `TableName`&gt;

#### 继承自 \{#inherited-from\}

BaseDatabaseReaderWithTable.table

#### 定义于 \{#defined-in\}

[server/database.ts:73](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L73)