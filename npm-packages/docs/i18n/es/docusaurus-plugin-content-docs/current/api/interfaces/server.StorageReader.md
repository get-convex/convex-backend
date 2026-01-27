---
id: "server.StorageReader"
title: "Interfaz: StorageReader"
custom_edit_url: null
---

[server](../modules/server.md).StorageReader

Una interfaz para leer archivos del almacenamiento dentro de funciones de consulta de Convex.

## Jerarquía \{#hierarchy\}

* **`StorageReader`**

  ↳ [`StorageWriter`](server.StorageWriter.md)

## Métodos \{#methods\}

### getUrl \{#geturl\}

▸ **getUrl**(`storageId`): `Promise`&lt;`null` | `string`&gt;

Obtiene la URL de un archivo en el almacenamiento por su `Id<"_storage">`.

La respuesta GET incluye una cabecera HTTP Digest estándar con un checksum sha256.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | El `Id<"_storage">` del archivo que se va a obtener del almacenamiento de Convex. |

#### Devuelve \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* Una URL para obtener el archivo mediante una solicitud HTTP GET, o `null` si ya no existe.

#### Definido en \{#defined-in\}

[server/storage.ts:51](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L51)

▸ **getUrl**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | `string`&gt;

**`Deprecated`**

Proporcionar una cadena de texto está obsoleto, usa `storage.getUrl(Id<"_storage">)` en su lugar.

Obtiene la URL de un archivo en el almacenamiento mediante su [StorageId](../modules/server.md#storageid).

La respuesta GET incluye una cabecera HTTP Digest estándar con una suma de comprobación sha256.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extiende `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | El [StorageId](../modules/server.md#storageid) del archivo que se va a recuperar del almacenamiento de Convex. |

#### Devuelve \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* Una URL que recupera el archivo mediante una solicitud HTTP GET, o `null` si ya no existe.

#### Definido en \{#defined-in\}

[server/storage.ts:63](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L63)

***

### getMetadata \{#getmetadata\}

▸ **getMetadata**(`storageId`): `Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

**`Deprecated`**

Esta función está en desuso; utiliza `db.system.get(Id<"_storage">)` en su lugar.

Obtiene los metadatos de un archivo.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | El `Id<"_storage">` del archivo. |

#### Devuelve \{#returns\}

`Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

* Un objeto [FileMetadata](../modules/server.md#filemetadata) si existe o `null` en caso contrario.

#### Definido en \{#defined-in\}

[server/storage.ts:75](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L75)

▸ **getMetadata**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

**`Deprecated`**

Esta función está en desuso, usa `db.system.get(Id<"_storage">)` en su lugar.

Obtiene los metadatos de un archivo.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extiende `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | El [StorageId](../modules/server.md#storageid) del archivo. |

#### Devuelve \{#returns\}

`Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

* Un objeto [FileMetadata](../modules/server.md#filemetadata) si se encuentra, o `null` si no se encuentra.

#### Definido en \{#defined-in\}

[server/storage.ts:85](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L85)