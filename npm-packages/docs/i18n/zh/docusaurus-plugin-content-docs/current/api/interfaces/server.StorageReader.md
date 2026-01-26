---
id: "server.StorageReader"
title: "接口：StorageReader"
custom_edit_url: null
---

[server](../modules/server.md).StorageReader

用于在 Convex 查询函数中从存储读取文件的接口。

## 层次结构 \{#hierarchy\}

* **`StorageReader`**

  ↳ [`StorageWriter`](server.StorageWriter.md)

## 方法 \{#methods\}

### getUrl \{#geturl\}

▸ **getUrl**(`storageId`): `Promise`&lt;`null` | `string`&gt;

通过存储中该文件的 `Id<"_storage">` 获取文件的 URL。

GET 响应中会包含带有 SHA-256 校验和的标准 HTTP Digest 头部。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | 用于从 Convex 存储中获取文件的 `Id<"_storage">`。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* 一个用于通过 HTTP GET 请求获取该文件的 URL；如果文件已不存在，则为 `null`。

#### 定义于 \{#defined-in\}

[server/storage.ts:51](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L51)

▸ **getUrl**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | `string`&gt;

**`已弃用`**

传入字符串参数的方式已弃用，请改用 `storage.getUrl(Id<"_storage">)`。

通过其 [StorageId](../modules/server.md#storageid) 获取存储中的文件 URL。

GET 响应会包含带有 sha256 校验和的标准 HTTP Digest 头部。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | extends `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | 要从 Convex 存储中获取的文件在存储中的 [StorageId](../modules/server.md#storageid)。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* 一个用于通过 HTTP GET 获取该文件的 URL，如果文件已不存在，则为 `null`。

#### 定义于 \{#defined-in\}

[server/storage.ts:63](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L63)

***

### getMetadata \{#getmetadata\}

▸ **getMetadata**(`storageId`): `Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

**`Deprecated`**

此函数已被弃用，请改用 `db.system.get(Id<"_storage">)`。

获取文件的元数据。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | 该文件的 `Id<"_storage">`。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

* 若找到则返回一个 [FileMetadata](../modules/server.md#filemetadata) 对象，否则返回 `null`。

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
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | 文件的 [StorageId](../modules/server.md#storageid)。 |

#### 返回值 \{#returns\}

`Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

* 如果找到则返回 [FileMetadata](../modules/server.md#filemetadata) 对象，否则返回 `null`。

#### 定义于 \{#defined-in\}

[server/storage.ts:85](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L85)