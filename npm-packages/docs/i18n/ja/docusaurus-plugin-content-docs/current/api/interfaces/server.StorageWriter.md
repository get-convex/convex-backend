---
id: "server.StorageWriter"
title: "インターフェース: StorageWriter"
custom_edit_url: null
---

[server](../modules/server.md).StorageWriter

Convex のミューテーション関数内からストレージにファイルを書き込むためのインターフェースです。

## 階層構造 \{#hierarchy\}

* [`StorageReader`](server.StorageReader.md)

  ↳ **`StorageWriter`**

  ↳↳ [`StorageActionWriter`](server.StorageActionWriter.md)

## メソッド \{#methods\}

### getUrl \{#geturl\}

▸ **getUrl**(`storageId`): `Promise`&lt;`null` | `string`&gt;

`Id<"_storage">` を指定して、ストレージ内のファイルの URL を取得します。

GET レスポンスには、sha256 チェックサムを含む標準的な HTTP Digest ヘッダーが含まれます。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | Convex ストレージから取得するファイルの `Id<"_storage">`。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* HTTP GET リクエストでファイルを取得するための URL。ファイルがすでに存在しない場合は `null`。

#### 継承元 \{#inherited-from\}

[StorageReader](server.StorageReader.md).[getUrl](server.StorageReader.md#geturl)

#### Defined in \{#defined-in\}

[server/storage.ts:51](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L51)

▸ **getUrl**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | `string`&gt;

**`Deprecated`**

文字列を渡すことは非推奨です。代わりに `storage.getUrl(Id<"_storage">)` を使用してください。

ストレージ内のファイルの [StorageId](../modules/server.md#storageid) から URL を取得します。

GET レスポンスには、sha256 チェックサムを含む標準的な HTTP Digest ヘッダーが付与されます。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends `string` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | Convex ストレージから取得するファイルの [StorageId](../modules/server.md#storageid)。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* ファイルを HTTP GET リクエストで取得するための URL。ファイルが存在しない場合は `null`。

#### 継承元 \{#inherited-from\}

[StorageReader](server.StorageReader.md).[getUrl](server.StorageReader.md#geturl)

#### 定義場所 \{#defined-in\}

[server/storage.ts:63](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L63)

***

### getMetadata \{#getmetadata\}

▸ **getMetadata**(`storageId`): `Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

**`Deprecated`**

この関数は非推奨です。代わりに `db.system.get(Id<"_storage">)` を使用してください。

ファイルのメタデータを取得します。

#### パラメーター \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | ファイルの `Id<"_storage">`。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

* 見つかった場合は [FileMetadata](../modules/server.md#filemetadata) オブジェクト、見つからなかった場合は `null`。

#### 継承元 \{#inherited-from\}

[StorageReader](server.StorageReader.md).[getMetadata](server.StorageReader.md#getmetadata)

#### 定義場所 \{#defined-in\}

[server/storage.ts:75](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L75)

▸ **getMetadata**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

**`非推奨`**

この関数は非推奨です。代わりに `db.system.get(Id<"_storage">)` を使用してください。

ファイルのメタデータを取得します。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends `string` |

#### パラメータ \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | ファイルの [StorageId](../modules/server.md#storageid)。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

* 見つかった場合は [FileMetadata](../modules/server.md#filemetadata) オブジェクト、見つからなかった場合は `null` を返します。

#### 継承元 \{#inherited-from\}

[StorageReader](server.StorageReader.md).[getMetadata](server.StorageReader.md#getmetadata)

#### 定義元 \{#defined-in\}

[server/storage.ts:85](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L85)

***

### generateUploadUrl \{#generateuploadurl\}

▸ **generateUploadUrl**(): `Promise`&lt;`string`&gt;

ストレージにファイルをアップロードするための有効期限付き URL を取得します。

この URL に対して POST リクエストを送信すると、エンドポイントは新しく割り当てられた `Id<"_storage">` を含む JSON オブジェクトを返します。

この POST 用 URL では、sha256 チェックサムを含む標準的な HTTP Digest ヘッダーをオプションで指定できます。

#### 戻り値 \{#returns\}

`Promise`&lt;`string`&gt;

* HTTP POST 経由でファイルをアップロードするための URL。

#### 定義元 \{#defined-in\}

[server/storage.ts:105](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L105)

***

### delete \{#delete\}

▸ **delete**(`storageId`): `Promise`&lt;`void`&gt;

Convex ストレージからファイルを削除します。

一度ファイルを削除すると、以前に [getUrl](server.StorageReader.md#geturl) によって生成された URL はすべて 404 を返すようになります。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | Convex ストレージから削除する対象ファイルの `Id<"_storage">`。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定義場所 \{#defined-in\}

[server/storage.ts:113](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L113)

▸ **delete**&lt;`T`&gt;(`storageId`): `Promise`&lt;`void`&gt;

**`Deprecated`**

`storageId` に文字列を渡すことは非推奨です。代わりに `storage.delete(Id<"_storage">)` を使用してください。

Convex ストレージからファイルを削除します。

一度ファイルが削除されると、以前に [getUrl](server.StorageReader.md#geturl) によって生成された URL は 404 を返すようになります。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends `string` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | Convex のストレージから削除するファイルの [StorageId](../modules/server.md#storageid)。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定義場所 \{#defined-in\}

[server/storage.ts:124](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L124)