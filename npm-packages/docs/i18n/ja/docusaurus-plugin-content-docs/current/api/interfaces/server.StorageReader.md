---
id: "server.StorageReader"
title: "インターフェイス: StorageReader"
custom_edit_url: null
---

[server](../modules/server.md).StorageReader

Convex のクエリ関数内でストレージ内のファイルを読み取るためのインターフェイスです。

## 継承階層 \{#hierarchy\}

* **`StorageReader`**

  ↳ [`StorageWriter`](server.StorageWriter.md)

## メソッド \{#methods\}

### getUrl \{#geturl\}

▸ **getUrl**(`storageId`): `Promise`&lt;`null` | `string`&gt;

`Id<"_storage">` によって指定されたストレージ内のファイルの URL を取得します。

GET のレスポンスには、sha256 チェックサムを含む標準的な HTTP Digest ヘッダーが含まれます。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | Convex ストレージから取得するファイルの `Id<"_storage">`。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* ファイルを HTTP GET リクエストで取得するための URL。ファイルが既に存在しない場合は `null`。

#### 定義元 \{#defined-in\}

[server/storage.ts:51](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L51)

▸ **getUrl**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | `string`&gt;

**`非推奨`**

文字列を渡すのは非推奨です。代わりに `storage.getUrl(Id<"_storage">)` を使用してください。

ストレージ内のファイルに対する URL を、その [StorageId](../modules/server.md#storageid) から取得します。

GET レスポンスには、sha256 チェックサムを含む標準的な HTTP Digest ヘッダーが付与されます。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | `string` を拡張 |

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | Convex のストレージから取得するファイルの [StorageId](../modules/server.md#storageid)。 |

#### Returns \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* HTTP GET リクエストでファイルを取得するための URL。ファイルがもはや存在しない場合は `null`。

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

#### Returns \{#returns\}

`Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

* 見つかった場合は [FileMetadata](../modules/server.md#filemetadata) オブジェクトを、見つからなかった場合は `null` を返します。

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

#### パラメーター \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | ファイルの [StorageId](../modules/server.md#storageid)。 |

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

* 見つかった場合は [FileMetadata](../modules/server.md#filemetadata) オブジェクトを、見つからなかった場合は `null` を返します。

#### 定義場所 \{#defined-in\}

[server/storage.ts:85](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L85)