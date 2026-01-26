---
id: "server.StorageActionWriter"
title: "接口: StorageActionWriter"
custom_edit_url: null
---

[server](../modules/server.md).StorageActionWriter

一个用于在 Convex 操作函数和 HTTP 操作函数中读写存储中文件的接口。

## 继承关系 \{#hierarchy\}

* [`StorageWriter`](server.StorageWriter.md)

  ↳ **`StorageActionWriter`**

## 方法 \{#methods\}

### getUrl \{#geturl\}

▸ **getUrl**(`storageId`): `Promise`&lt;`null` | `string`&gt;

通过存储中文件的 `Id<"_storage">` 获取该文件的 URL。

GET 响应包含一个标准的 HTTP Digest 首部，其中带有 sha256 校验和。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | 要从 Convex 存储中获取的文件的 `Id<"_storage">`。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* 一个用于通过 HTTP GET 获取该文件的 URL；如果文件已不存在，则返回 `null`。

#### 继承自 \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[getUrl](server.StorageWriter.md#geturl)

#### 定义于 \{#defined-in\}

[server/storage.ts:51](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L51)

▸ **getUrl**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | `string`&gt;

**`Deprecated`**

传入字符串的方式已被弃用，请改用 `storage.getUrl(Id<"_storage">)`。

通过其 [StorageId](../modules/server.md#storageid) 获取存储中的文件 URL。

GET 响应会包含一个标准的 HTTP Digest 头部，其中包含 sha256 校验和。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | 要从 Convex 存储中获取的文件的 [StorageId](../modules/server.md#storageid)。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* 一个用于通过 HTTP GET 获取该文件的 URL，如果文件已不存在则为 `null`。

#### 继承自 \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[getUrl](server.StorageWriter.md#geturl)

#### 定义于 \{#defined-in\}

[server/storage.ts:63](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L63)

***

### getMetadata \{#getmetadata\}

▸ **getMetadata**(`storageId`): `Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

**`Deprecated`**

此函数已弃用，请改用 `db.system.get(Id<"_storage">)`。

获取文件的元数据。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | 该文件的 `Id<"_storage">`。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

* 如果找到则返回一个 [FileMetadata](../modules/server.md#filemetadata) 对象，否则返回 `null`。

#### 继承自 \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[getMetadata](server.StorageWriter.md#getmetadata)

#### 定义于 \{#defined-in\}

[server/storage.ts:75](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L75)

▸ **getMetadata**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

**`已弃用`**

此函数已弃用，请改用 `db.system.get(Id<"_storage">)`。

获取文件元数据。

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

[StorageWriter](server.StorageWriter.md).[getMetadata](server.StorageWriter.md#getmetadata)

#### 定义于 \{#defined-in\}

[server/storage.ts:85](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L85)

***

### generateUploadUrl \{#generateuploadurl\}

▸ **generateUploadUrl**(): `Promise`&lt;`string`&gt;

获取一个用于将文件上传到存储中的短时有效 URL。

对该 URL 发起 POST 请求时，端点会返回一个 JSON 对象，其中包含新分配的 `Id<"_storage">`。

该 POST URL 支持可选的标准 HTTP Digest 头部，其中包含 SHA-256 校验和。

#### 返回值 \{#returns\}

`Promise`&lt;`string`&gt;

* 用于通过 HTTP POST 上传文件的 URL。

#### 继承自 \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[generateUploadUrl](server.StorageWriter.md#generateuploadurl)

#### 定义于 \{#defined-in\}

[server/storage.ts:105](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L105)

***

### delete \{#delete\}

▸ **delete**(`storageId`): `Promise`&lt;`void`&gt;

从 Convex 存储中删除一个文件。

文件被删除后，此前由 [getUrl](server.StorageReader.md#geturl) 生成的任何 URL 都会返回 404。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | 在 Convex 存储中要删除的文件的 `Id<"_storage">`。 |

#### 返回值 \{#returns\}

`Promise`&lt;`void`&gt;

#### 继承自 \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[delete](server.StorageWriter.md#delete)

#### 定义于 \{#defined-in\}

[server/storage.ts:113](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L113)

▸ **delete**&lt;`T`&gt;(`storageId`): `Promise`&lt;`void`&gt;

**`已弃用`**

传入字符串的用法已弃用，请改用 `storage.delete(Id<"_storage">)`。

从 Convex 存储中删除一个文件。

一旦文件被删除，之前通过 [getUrl](server.StorageReader.md#geturl) 生成的任何 URL 都将返回 404。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | 要从 Convex 存储中删除的文件对应的 [StorageId](../modules/server.md#storageid)。 |

#### 返回值 \{#returns\}

`Promise`&lt;`void`&gt;

#### 继承自 \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[delete](server.StorageWriter.md#delete)

#### 定义于 \{#defined-in\}

[server/storage.ts:124](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L124)

***

### get \{#get\}

▸ **get**(`storageId`): `Promise`&lt;`null` | `Blob`&gt;

获取一个包含与所提供的 `Id<"_storage">` 关联的文件的 Blob；如果没有对应文件，则返回 `null`。

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; |

#### 返回 \{#returns\}

`Promise`&lt;`null` | `Blob`&gt;

#### 定义于 \{#defined-in\}

[server/storage.ts:138](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L138)

▸ **get**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | `Blob`&gt;

**`已弃用`**

传入字符串的方式已弃用，请改用 `storage.get(Id<"_storage">)`。

获取一个包含与提供的 [StorageId](../modules/server.md#storageid) 关联文件的 Blob；如果没有文件则返回 `null`。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | `Blob`&gt;

#### 定义于 \{#defined-in\}

[server/storage.ts:145](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L145)

***

### store \{#store\}

▸ **store**(`blob`, `options?`): `Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt;&gt;

存储 Blob 中包含的文件。

如果提供该参数，将会验证提供的 sha256 校验和是否与文件内容匹配。

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `blob` | `Blob` |
| `options?` | `Object` |
| `options.sha256?` | `string` |

#### 返回值 \{#returns\}

`Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt;&gt;

#### 定义于 \{#defined-in\}

[server/storage.ts:153](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L153)