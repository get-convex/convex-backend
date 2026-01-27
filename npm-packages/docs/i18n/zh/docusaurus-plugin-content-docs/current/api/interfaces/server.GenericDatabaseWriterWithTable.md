---
id: "server.GenericDatabaseWriterWithTable"
title: "接口：GenericDatabaseWriterWithTable<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericDatabaseWriterWithTable

一个在 Convex 变更函数中对数据库进行读写的接口。

Convex 保证单个变更中的所有写入都是原子执行的，因此你无需担心部分写入会让数据处于不一致状态。有关 Convex 为你的函数提供的这些保证，请参阅 [Convex 指南](https://docs.convex.dev/understanding/convex-fundamentals/functions#atomicity-and-optimistic-concurrency-control)。

如果你在使用代码生成，请在 `convex/_generated/server.d.ts` 中使用为你的数据模型提供类型定义的 `DatabaseReader` 类型。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | 继承自 [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## 继承结构 \{#hierarchy\}

* [`GenericDatabaseReaderWithTable`](server.GenericDatabaseReaderWithTable.md)&lt;`DataModel`&gt;

  ↳ **`GenericDatabaseWriterWithTable`**

## 属性 \{#properties\}

### system \{#system\}

• **system**: `BaseDatabaseReaderWithTable`&lt;[`SystemDataModel`](server.SystemDataModel.md)&gt;

在 Convex 查询函数中用于访问系统表的接口

有两个入口点：

* [get](server.GenericDatabaseReader.md#get)，根据其 [GenericId](../modules/values.md#genericid) 获取单个文档。
  * [query](server.GenericDatabaseReader.md#query)，用于开始构建查询。

#### 继承自 \{#inherited-from\}

[GenericDatabaseReaderWithTable](server.GenericDatabaseReaderWithTable.md).[system](server.GenericDatabaseReaderWithTable.md#system)

#### 定义于 \{#defined-in\}

[server/database.ts:144](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L144)

## 方法 \{#methods\}

### table \{#table\}

▸ **table**&lt;`TableName`&gt;(`tableName`): [`BaseTableWriter`](server.BaseTableWriter.md)&lt;`DataModel`, `TableName`&gt;

将数据库操作限定到某个特定数据表。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `tableName` | `TableName` |

#### 返回值 \{#returns\}

[`BaseTableWriter`](server.BaseTableWriter.md)&lt;`DataModel`, `TableName`&gt;

#### 重写 \{#overrides\}

[GenericDatabaseReaderWithTable](server.GenericDatabaseReaderWithTable.md).[table](server.GenericDatabaseReaderWithTable.md#table)

#### 定义于 \{#defined-in\}

[server/database.ts:274](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L274)