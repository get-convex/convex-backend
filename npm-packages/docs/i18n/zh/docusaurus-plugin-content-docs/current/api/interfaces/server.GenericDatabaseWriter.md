---
id: "server.GenericDatabaseWriter"
title: "接口：GenericDatabaseWriter<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericDatabaseWriter

在 Convex 的变更函数中，用于读写数据库的接口。

Convex 保证单个变更中的所有写入都是原子执行的，因此你不必担心部分写入会使数据处于不一致状态。参见 [Convex 指南](https://docs.convex.dev/understanding/convex-fundamentals/functions#atomicity-and-optimistic-concurrency-control) 了解 Convex 为你的函数提供的这些保证。

如果你在使用代码生成，请在 `convex/_generated/server.d.ts` 中使用按你的数据模型进行了类型定义的 `DatabaseReader` 类型。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | 继承自 [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## 层次结构 \{#hierarchy\}

* [`GenericDatabaseReader`](server.GenericDatabaseReader.md)&lt;`DataModel`&gt;

  ↳ **`GenericDatabaseWriter`**

## 属性 \{#properties\}

### system \{#system\}

• **system**: `BaseDatabaseReader`&lt;[`SystemDataModel`](server.SystemDataModel.md)&gt;

用于在 Convex 查询函数中读取系统表的接口

有两个主要入口：

* [get](server.GenericDatabaseReader.md#get)，根据其 [GenericId](../modules/values.md#genericid) 获取单个文档。
  * [query](server.GenericDatabaseReader.md#query)，用于开始构建一个查询。

#### 继承自 \{#inherited-from\}

[GenericDatabaseReader](server.GenericDatabaseReader.md).[system](server.GenericDatabaseReader.md#system)

#### 定义于 \{#defined-in\}

[server/database.ts:128](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L128)

## 方法 \{#methods\}

### get \{#get\}

▸ **get**&lt;`TableName`&gt;(`table`, `id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

根据其 [GenericId](../modules/values.md#genericid) 从数据库中获取单个文档。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 说明 |
| :------ | :------ | :------ |
| `table` | `TableName` | 要从中读取文档的表名。 |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`NonUnion`&lt;`TableName`&gt;&gt; | 要从数据库读取的文档的 [GenericId](../modules/values.md#genericid)。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* 给定 [GenericId](../modules/values.md#genericid) 对应文档的 [GenericDocument](../modules/server.md#genericdocument)，如果该文档已不存在则返回 `null`。

#### 继承自 \{#inherited-from\}

[GenericDatabaseReader](server.GenericDatabaseReader.md).[get](server.GenericDatabaseReader.md#get)

#### 定义于 \{#defined-in\}

[server/database.ts:23](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L23)

▸ **get**&lt;`TableName`&gt;(`id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

通过其 [GenericId](../modules/values.md#genericid) 从数据库获取单个文档。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | 要从数据库中读取的文档的 [GenericId](../modules/values.md#genericid)。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* 给定 [GenericId](../modules/values.md#genericid) 对应文档的 [GenericDocument](../modules/server.md#genericdocument)；如果该文档已不存在，则返回 `null`。

#### 继承自 \{#inherited-from\}

[GenericDatabaseReader](server.GenericDatabaseReader.md).[get](server.GenericDatabaseReader.md#get)

#### 定义于 \{#defined-in\}

[server/database.ts:34](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L34)

***

### 查询 \{#query\}

▸ **query**&lt;`TableName`&gt;(`tableName`): [`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

为给定的表名开始一个查询。

查询不会立即执行，因此在结果真正被使用之前，调用此方法并在其基础上继续扩展查询都是没有开销的。

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

* 一个用于开始构建查询的[QueryInitializer](server.QueryInitializer.md)对象。

#### 继承自 \{#inherited-from\}

[GenericDatabaseReader](server.GenericDatabaseReader.md).[query](server.GenericDatabaseReader.md#query)

#### 定义于 \{#defined-in\}

[server/database.ts:47](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L47)

***

### normalizeId \{#normalizeid\}

▸ **normalizeId**&lt;`TableName`&gt;(`tableName`, `id`): `null` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;

返回给定数据表中该 ID 的字符串形式，如果该 ID 来自其他数据表或不是有效的 ID，则返回 null。

此方法既接受字符串形式的 ID，也接受旧版基于类的 ID 格式的 `.toString()` 字符串。

这并不保证该 ID 对应的文档实际存在（即 `db.get(id)` 可能返回 `null`）。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `tableName` | `TableName` | 表名。 |
| `id` | `string` | ID 字符串。 |

#### 返回值 \{#returns\}

`null` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;

#### 继承自 \{#inherited-from\}

[GenericDatabaseReader](server.GenericDatabaseReader.md).[normalizeId](server.GenericDatabaseReader.md#normalizeid)

#### 定义于 \{#defined-in\}

[server/database.ts:63](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L63)

***

### insert \{#insert\}

▸ **insert**&lt;`TableName`&gt;(`table`, `value`): `Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;&gt;

向表中插入一个新文档。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `table` | `TableName` | 要插入新文档的表名。 |
| `value` | [`WithoutSystemFields`](../modules/server.md#withoutsystemfields)&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | 要插入到指定表中的[值](../modules/values.md#value)。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;&gt;

* 新创建文档的 [GenericId](../modules/values.md#genericid)。

#### 定义于 \{#defined-in\}

[server/database.ts:170](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L170)

***

### patch \{#patch\}

▸ **patch**&lt;`TableName`&gt;(`table`, `id`, `value`): `Promise`&lt;`void`&gt;

对已有文档进行部分更新，将其与给定的部分文档进行浅层合并。

新字段会被添加。已有字段会被覆盖。被设为
`undefined` 的字段会被移除。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `table` | `TableName` | 文档所在数据表的名称。 |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`NonUnion`&lt;`TableName`&gt;&gt; | 要进行局部更新的文档的 [GenericId](../modules/values.md#genericid)。 |
| `value` | `PatchValue`&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | 要合并到指定文档中的部分 [GenericDocument](../modules/server.md#genericdocument)。如果此新值指定了 `_id` 等系统字段，它们必须与该文档现有字段值完全一致。 |

#### 返回 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定义于 \{#defined-in\}

[server/database.ts:187](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L187)

▸ **patch**&lt;`TableName`&gt;(`id`, `value`): `Promise`&lt;`void`&gt;

对已有文档进行局部更新，将其与给定的部分文档进行浅合并。

会添加新的字段。已有字段会被覆盖。被设置为
`undefined` 的字段会被删除。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | 要部分更新的文档的 [GenericId](../modules/values.md#genericid)。 |
| `value` | `PatchValue`&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | 要合并到指定文档中的部分 [GenericDocument](../modules/server.md#genericdocument)。如果该新值包含 `_id` 等系统字段，则这些字段的值必须与该文档现有的字段值一致。 |

#### 返回值 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定义于 \{#defined-in\}

[server/database.ts:204](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L204)

***

### replace \{#replace\}

▸ **replace**&lt;`TableName`&gt;(`table`, `id`, `value`): `Promise`&lt;`void`&gt;

替换现有文档的值，覆盖其旧值。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `table` | `TableName` | 文档所在表的名称。 |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`NonUnion`&lt;`TableName`&gt;&gt; | 要替换的文档的 [GenericId](../modules/values.md#genericid)。 |
| `value` | [`WithOptionalSystemFields`](../modules/server.md#withoptionalsystemfields)&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | 文档新的 [GenericDocument](../modules/server.md#genericdocument)。此值可以省略系统字段，数据库会自动补全这些字段。 |

#### 返回 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定义于 \{#defined-in\}

[server/database.ts:217](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L217)

▸ **replace**&lt;`TableName`&gt;(`id`, `value`): `Promise`&lt;`void`&gt;

替换现有文档的值，覆盖其原有值。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | 要替换的文档的 [GenericId](../modules/values.md#genericid)。 |
| `value` | [`WithOptionalSystemFields`](../modules/server.md#withoptionalsystemfields)&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | 该文档新的 [GenericDocument](../modules/server.md#genericdocument)。此值可以省略系统字段，数据库会自动填充这些字段。 |

#### 返回值 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定义于 \{#defined-in\}

[server/database.ts:230](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L230)

***

### delete \{#delete\}

▸ **delete**&lt;`TableName`&gt;(`table`, `id`): `Promise`&lt;`void`&gt;

删除已有文档。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `table` | `TableName` | 文档所在数据表的名称。 |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`NonUnion`&lt;`TableName`&gt;&gt; | 要删除的文档的 [GenericId](../modules/values.md#genericid)。 |

#### 返回值 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定义于 \{#defined-in\}

[server/database.ts:241](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L241)

▸ **delete**(`id`): `Promise`&lt;`void`&gt;

删除一条现有文档。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;[`TableNamesInDataModel`](../modules/server.md#tablenamesindatamodel)&lt;`DataModel`&gt;&gt; | 要删除的文档的 [GenericId](../modules/values.md#genericid)。 |

#### 返回值 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定义于 \{#defined-in\}

[server/database.ts:251](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L251)