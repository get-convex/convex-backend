---
id: "server.StorageWriter"
title: "接口：StorageWriter"
custom_edit_url: null
---

[server](../modules/server.md).StorageWriter

用于在 Convex 的变更函数中向存储写入文件的接口。

## 层级结构 \{#hierarchy\}

* [`StorageReader`](server.StorageReader.md)

  ↳ **`StorageWriter`**

  ↳↳ [`StorageActionWriter`](server.StorageActionWriter.md)

## 方法 \{#methods\}

### getUrl \{#geturl\}

▸ **getUrl**(`storageId`): `Promise`&lt;`null` | `string`&gt;

通过其在存储中的 `Id<"_storage">` 获取文件的 URL。

该 GET 响应包含一个带有 sha256 校验和的标准 HTTP Digest 头部。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | 要从 Convex 存储中获取的文件的 `Id<"_storage">`。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* 一个用于通过 HTTP GET 获取该文件的 URL；如果文件不再存在，则为 `null`。

#### 继承自 \{#inherited-from\}

[StorageReader](server.StorageReader.md).[getUrl](server.StorageReader.md#geturl)

#### 定义于 \{#defined-in\}

[server/storage.ts:51](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L51)

▸ **getUrl**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | `string`&gt;

**`已弃用`**

将字符串作为参数传入的用法已弃用，请改用 `storage.getUrl(Id<"_storage">)`。

通过其 [StorageId](../modules/server.md#storageid) 获取存储中文件的 URL。

GET 响应包含一个标准的 HTTP Digest 头部，其中带有 sha256 校验和。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | 用于从 Convex 存储中获取文件的 [StorageId](../modules/server.md#storageid)。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* 一个用于通过 HTTP GET 获取该文件的 URL；如果文件已不存在，则返回 `null`。

#### 继承自 \{#inherited-from\}

[StorageReader](server.StorageReader.md).[getUrl](server.StorageReader.md#geturl)

#### 定义于 \{#defined-in\}

[server/storage.ts:63](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L63)

***

### getMetadata \{#getmetadata\}

▸ **getMetadata**(`storageId`): `Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

**`Deprecated`**

此函数已弃用，请改为使用 `db.system.get(Id<"_storage">)`。

获取某个文件的元数据。

#### 参数 \{#parameters\}

| 名称 | 类型 | 说明 |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | 文件的 `Id<"_storage">`。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

* 如果找到则返回一个 [FileMetadata](../modules/server.md#filemetadata) 对象，否则返回 `null`。

#### 继承自 \{#inherited-from\}

[StorageReader](server.StorageReader.md).[getMetadata](server.StorageReader.md#getmetadata)

#### 定义于 \{#defined-in\}

[server/storage.ts:75](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L75)

▸ **getMetadata**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

**`已弃用`**

此函数已弃用，请改用 `db.system.get(Id<"_storage">)`。

获取文件的元数据。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | 文件对应的 [StorageId](../modules/server.md#storageid)。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

* 找到时返回一个 [FileMetadata](../modules/server.md#filemetadata) 对象，未找到时返回 `null`。

#### 继承自 \{#inherited-from\}

[StorageReader](server.StorageReader.md).[getMetadata](server.StorageReader.md#getmetadata)

#### 定义于 \{#defined-in\}

[server/storage.ts:85](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L85)

***

### generateUploadUrl \{#generateuploadurl\}

▸ **generateUploadUrl**(): `Promise`&lt;`string`&gt;

获取一个用于将文件上传到存储中的短时有效 URL。

向该 URL 发起 POST 请求后，端点会返回一个 JSON 对象，其中包含新分配的 `Id<"_storage">`。

该 POST URL 接受一个可选的标准 HTTP Digest 头，其中包含 sha256 校验和。

#### 返回值 \{#returns\}

`Promise`&lt;`string`&gt;

* 允许通过 HTTP POST 上传文件的 URL。

#### 定义于 \{#defined-in\}

[server/storage.ts:105](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L105)

***

### delete \{#delete\}

▸ **delete**(`storageId`): `Promise`&lt;`void`&gt;

从 Convex 存储中删除文件。

一旦文件被删除，之前由 [getUrl](server.StorageReader.md#geturl) 生成的任何 URL 都会返回 404 状态码。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | 要从 Convex 存储中删除的文件的 `Id<"_storage">`。 |

#### 返回 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定义于 \{#defined-in\}

[server/storage.ts:113](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L113)

▸ **delete**&lt;`T`&gt;(`storageId`): `Promise`&lt;`void`&gt;

**`已弃用`**

传入字符串的用法已弃用，请改用 `storage.delete(Id<"_storage">)`。

从 Convex 存储中删除文件。

文件被删除后，之前由 [getUrl](server.StorageReader.md#geturl) 生成的任何 URL 都将返回 404。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | 表示要从 Convex 存储中删除的文件的 [StorageId](../modules/server.md#storageid)。 |

#### 返回 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定义于 \{#defined-in\}

[server/storage.ts:124](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L124)