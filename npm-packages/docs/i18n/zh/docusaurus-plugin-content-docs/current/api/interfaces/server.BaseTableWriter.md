---
id: "server.BaseTableWriter"
title: "接口：BaseTableWriter<DataModel, TableName>"
custom_edit_url: null
---

[server](../modules/server.md).BaseTableWriter

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | 继承自 [`GenericDataModel`](../modules/server.md#genericdatamodel) |
| `TableName` | 继承自 [`TableNamesInDataModel`](../modules/server.md#tablenamesindatamodel)&lt;`DataModel`&gt; |

## 继承层次结构 \{#hierarchy\}

* [`BaseTableReader`](server.BaseTableReader.md)&lt;`DataModel`, `TableName`&gt;

  ↳ **`BaseTableWriter`**

## 方法 \{#methods\}

### get \{#get\}

▸ **get**(`id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

根据其 [GenericId](../modules/values.md#genericid) 从表中获取单个文档。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | 要从数据库获取的文档的 [GenericId](../modules/values.md#genericid)。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* 给定 [GenericId](../modules/values.md#genericid) 对应文档的 [GenericDocument](../modules/server.md#genericdocument)，如果该文档已不存在，则返回 `null`。

#### 继承自 \{#inherited-from\}

[BaseTableReader](server.BaseTableReader.md).[get](server.BaseTableReader.md#get)

#### 定义于 \{#defined-in\}

[server/database.ts:88](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L88)

***

### query \{#query\}

▸ **query**(): [`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

开始为该表构建一个查询。

查询不会立即执行，因此在结果真正被使用之前，调用此方法并在其基础上继续扩展查询是没有开销的。

#### 返回值 \{#returns\}

[`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

* 用于开始构建查询的 [QueryInitializer](server.QueryInitializer.md) 对象。

#### 继承自 \{#inherited-from\}

[BaseTableReader](server.BaseTableReader.md).[query](server.BaseTableReader.md#query)

#### 定义在 \{#defined-in\}

[server/database.ts:100](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L100)

***

### insert \{#insert\}

▸ **insert**(`value`): `Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;&gt;

向该表插入一个新文档。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `value` | [`WithoutSystemFields`](../modules/server.md#withoutsystemfields)&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | 要插入到指定表中的[值](../modules/values.md#value)。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;&gt;

* 新建文档的 [GenericId](../modules/values.md#genericid)。

#### 定义于 \{#defined-in\}

[server/database.ts:289](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L289)

***

### patch \{#patch\}

▸ **patch**(`id`, `value`): `Promise`&lt;`void`&gt;

对现有文档进行局部更新，将其与给定的部分文档进行浅层合并。

新字段会被添加。已有字段会被覆盖。被设置为 `undefined` 的字段会被删除。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | 要打补丁的文档的 [GenericId](../modules/values.md#genericid)。 |
| `value` | `PatchValue`&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | 要合并到指定文档中的部分 [GenericDocument](../modules/server.md#genericdocument)。如果该新值中包含 `_id` 等系统字段，它们必须与文档中现有字段的值一致。 |

#### 返回 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定义于 \{#defined-in\}

[server/database.ts:304](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L304)

***

### replace \{#replace\}

▸ **replace**(`id`, `value`): `Promise`&lt;`void`&gt;

将现有文档的值替换为新值，覆盖其旧值。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | 要替换的文档的 [GenericId](../modules/values.md#genericid)。 |
| `value` | [`WithOptionalSystemFields`](../modules/server.md#withoptionalsystemfields)&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | 该文档的新 [GenericDocument](../modules/server.md#genericdocument)。此值可以省略系统字段，数据库会自动补全这些字段。 |

#### 返回值 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定义于 \{#defined-in\}

[server/database.ts:316](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L316)

***

### delete \{#delete\}

▸ **delete**(`id`): `Promise`&lt;`void`&gt;

删除现有文档。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | 要删除的文档的 [GenericId](../modules/values.md#genericid)。 |

#### 返回值 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定义于 \{#defined-in\}

[server/database.ts:326](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L326)