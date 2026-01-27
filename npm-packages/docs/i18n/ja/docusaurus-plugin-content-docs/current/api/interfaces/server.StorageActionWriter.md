---
id: "server.StorageActionWriter"
title: "インターフェース: StorageActionWriter"
custom_edit_url: null
---

[server](../modules/server.md).StorageActionWriter

Convex アクションおよび HTTP アクション内で、ストレージ内のファイルを読み書きするためのインターフェースです。

## 継承階層 \{#hierarchy\}

* [`StorageWriter`](server.StorageWriter.md)

  ↳ **`StorageActionWriter`**

## メソッド \{#methods\}

### getUrl \{#geturl\}

▸ **getUrl**(`storageId`): `Promise`&lt;`null` | `string`&gt;

`Id<"_storage">` を指定して、ストレージ内のファイルの URL を取得します。

GET レスポンスには、sha256 チェックサムを含む標準的な HTTP Digest ヘッダーが含まれます。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | Convex ストレージから取得する対象ファイルの `Id<"_storage">`。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* ファイルを HTTP GET リクエストで取得するための URL。ファイルが存在しない場合は `null`。

#### 継承元 \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[getUrl](server.StorageWriter.md#geturl)

#### 定義元 \{#defined-in\}

[server/storage.ts:51](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L51)

▸ **getUrl**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | `string`&gt;

**`Deprecated`**

文字列をそのまま渡す方法は非推奨です。代わりに `storage.getUrl(Id<"_storage">)` を使用してください。

[StorageId](../modules/server.md#storageid) を用いて、ストレージ内のファイルの URL を取得します。

GET レスポンスには、sha256 チェックサム付きの標準的な HTTP Digest ヘッダーが含まれます。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends `string` |

#### パラメーター \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | Convex ストレージから取得するファイルの [StorageId](../modules/server.md#storageid)。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* ファイルを HTTP GET で取得するための URL。ファイルが既に存在しない場合は `null`。

#### 継承元 \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[getUrl](server.StorageWriter.md#geturl)

#### 定義元 \{#defined-in\}

[server/storage.ts:63](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L63)

***

### getMetadata \{#getmetadata\}

▸ **getMetadata**(`storageId`): `Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

**`Deprecated`**

この関数は非推奨です。代わりに `db.system.get(Id<"_storage">)` を使用してください。

ファイルのメタデータを取得します。

#### パラメータ \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | ファイルの `Id<"_storage">`。 |

#### Returns \{#returns\}

`Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

* 見つかった場合は [FileMetadata](../modules/server.md#filemetadata) オブジェクトが返され、見つからなかった場合は `null` が返されます。

#### 継承元 \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[getMetadata](server.StorageWriter.md#getmetadata)

#### 定義場所 \{#defined-in\}

[server/storage.ts:75](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L75)

▸ **getMetadata**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

**`非推奨`**

この関数は非推奨です。代わりに `db.system.get(Id<"_storage">)` を使用してください。

ファイルのメタデータを取得します。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | `string` を拡張 |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | ファイルの [StorageId](../modules/server.md#storageid) を表します。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

* 見つかった場合は [FileMetadata](../modules/server.md#filemetadata) オブジェクトが、見つからなかった場合は `null` が返されます。

#### 継承元 \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[getMetadata](server.StorageWriter.md#getmetadata)

#### 定義場所 \{#defined-in\}

[server/storage.ts:85](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L85)

***

### generateUploadUrl \{#generateuploadurl\}

▸ **generateUploadUrl**(): `Promise`&lt;`string`&gt;

ストレージにファイルをアップロードするための短期間のみ有効な URL を取得します。

この URL に対して POST リクエストを送信すると、エンドポイントは新しく割り当てられた `Id<"_storage">` を含む JSON オブジェクトを返します。

この URL への POST リクエストでは、オプションで sha256 チェックサムを含む標準の HTTP Digest ヘッダーを受け付けます。

#### 戻り値 \{#returns\}

`Promise`&lt;`string`&gt;

* HTTP POST リクエスト経由でファイルをアップロードできる URL。

#### 継承元 \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[generateUploadUrl](server.StorageWriter.md#generateuploadurl)

#### 定義場所 \{#defined-in\}

[server/storage.ts:105](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L105)

***

### delete \{#delete\}

▸ **delete**(`storageId`): `Promise`&lt;`void`&gt;

Convex のストレージからファイルを削除します。

ファイルが削除されると、以前に [getUrl](server.StorageReader.md#geturl) によって生成された URL は 404 を返すようになります。

#### パラメータ \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | Convex ストレージから削除するファイルの `Id<"_storage">`。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`void`&gt;

#### 継承元 \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[delete](server.StorageWriter.md#delete)

#### 定義場所 \{#defined-in\}

[server/storage.ts:113](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L113)

▸ **delete**&lt;`T`&gt;(`storageId`): `Promise`&lt;`void`&gt;

**`Deprecated`**

文字列を渡すことは非推奨です。代わりに `storage.delete(Id<"_storage">)` を使用してください。

Convex ストレージからファイルを削除します。

ファイルが削除されると、以前に [getUrl](server.StorageReader.md#geturl) によって生成された任意の URL は 404 を返すようになります。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | `string` を拡張する |

#### パラメーター \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | Convex ストレージから削除するファイルの [StorageId](../modules/server.md#storageid)。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`void`&gt;

#### 継承元 \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[delete](server.StorageWriter.md#delete)

#### 定義箇所 \{#defined-in\}

[server/storage.ts:124](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L124)

***

### get \{#get\}

▸ **get**(`storageId`): `Promise`&lt;`null` | `Blob`&gt;

指定された `Id<"_storage">` に対応するファイルを含む Blob オブジェクトを取得します。ファイルが存在しない場合は `null` を返します。

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; |

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | `Blob`&gt;

#### 定義場所 \{#defined-in\}

[server/storage.ts:138](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L138)

▸ **get**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | `Blob`&gt;

**`Deprecated`**

文字列を渡すことは非推奨です。代わりに `storage.get(Id<"_storage">)` を使用してください。

指定された [StorageId](../modules/server.md#storageid) に関連付けられたファイルを含む Blob オブジェクトを取得します。ファイルが存在しない場合は `null` を返します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends `string` |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` |

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | `Blob`&gt;

#### 定義場所 \{#defined-in\}

[server/storage.ts:145](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L145)

***

### store \{#store\}

▸ **store**(`blob`, `options?`): `Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt;&gt;

Blob に含まれるファイルを保存します。

`options` が指定されている場合は、ファイルの内容と SHA-256 チェックサムが一致することを検証します。

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `blob` | `Blob` |
| `options?` | `Object` |
| `options.sha256?` | `string` |

#### 戻り値 \{#returns\}

`Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt;&gt;

#### 定義場所 \{#defined-in\}

[server/storage.ts:153](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L153)