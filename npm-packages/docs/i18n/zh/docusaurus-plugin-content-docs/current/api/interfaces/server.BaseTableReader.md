---
id: "server.BaseTableReader"
title: "接口：BaseTableReader<DataModel, TableName>"
custom_edit_url: null
---

[server](../modules/server.md).BaseTableReader

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | 继承自 [`GenericDataModel`](../modules/server.md#genericdatamodel) |
| `TableName` | 继承自 [`TableNamesInDataModel`](../modules/server.md#tablenamesindatamodel)&lt;`DataModel`&gt; |

## 继承层次结构 \{#hierarchy\}

* **`BaseTableReader`**

  ↳ [`BaseTableWriter`](server.BaseTableWriter.md)

## 方法 \{#methods\}

### get \{#get\}

▸ **get**(`id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

根据其 [GenericId](../modules/values.md#genericid) 从表中获取单条文档。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | 要从数据库中获取的文档的 [GenericId](../modules/values.md#genericid)。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* 给定 [GenericId](../modules/values.md#genericid) 所对应文档的 [GenericDocument](../modules/server.md#genericdocument)，如果该文档已不存在，则为 `null`。

#### 定义于 \{#defined-in\}

[server/database.ts:88](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L88)

***

### query \{#query\}

▸ **query**(): [`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

开始针对该表的查询。

查询不会立即执行，因此调用此方法并在此基础上继续构建该查询，在实际使用结果之前都是零成本的。

#### 返回值 \{#returns\}

[`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

* 一个用于开始构建查询的 [`QueryInitializer`](server.QueryInitializer.md) 对象。

#### 定义在 \{#defined-in\}

[server/database.ts:100](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L100)