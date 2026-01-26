---
id: "server.StorageWriter"
title: "Interfaz: StorageWriter"
custom_edit_url: null
---

[server](../modules/server.md).StorageWriter

Una interfaz para escribir archivos en el almacenamiento desde funciones de mutación de Convex.

## Jerarquía \{#hierarchy\}

* [`StorageReader`](server.StorageReader.md)

  ↳ **`StorageWriter`**

  ↳↳ [`StorageActionWriter`](server.StorageActionWriter.md)

## Métodos \{#methods\}

### getUrl \{#geturl\}

▸ **getUrl**(`storageId`): `Promise`&lt;`null` | `string`&gt;

Obtiene la URL de un archivo en el almacenamiento mediante su `Id<"_storage">`.

La respuesta GET incluye una cabecera HTTP Digest estándar con un checksum sha256.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | El `Id<"_storage">` del archivo que se va a recuperar del almacenamiento de Convex. |

#### Devuelve \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* Una URL para obtener el archivo mediante una solicitud HTTP GET, o `null` si ya no existe.

#### Heredado de \{#inherited-from\}

[StorageReader](server.StorageReader.md).[getUrl](server.StorageReader.md#geturl)

#### Definido en \{#defined-in\}

[server/storage.ts:51](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L51)

▸ **getUrl**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | `string`&gt;

**`Deprecated`**

El uso de una cadena está en desuso; utiliza `storage.getUrl(Id<"_storage">)` en su lugar.

Obtén la URL de un archivo en el almacenamiento por su [StorageId](../modules/server.md#storageid).

La respuesta GET incluye una cabecera HTTP Digest estándar con una suma de comprobación (checksum) sha256.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extends `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | El [StorageId](../modules/server.md#storageid) del archivo que se va a obtener del almacenamiento de Convex. |

#### Devuelve \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* Una URL que recupera el archivo mediante una solicitud HTTP GET, o `null` si ya no existe.

#### Heredado de \{#inherited-from\}

[StorageReader](server.StorageReader.md).[getUrl](server.StorageReader.md#geturl)

#### Definido en \{#defined-in\}

[server/storage.ts:63](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L63)

***

### getMetadata \{#getmetadata\}

▸ **getMetadata**(`storageId`): `Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

**`Deprecated`**

Esta función está en desuso, usa `db.system.get(Id<"_storage">)` en su lugar.

Obtiene los metadatos de un archivo.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | El `Id<"_storage">` del archivo. |

#### Devuelve \{#returns\}

`Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

* Un objeto [FileMetadata](../modules/server.md#filemetadata) si existe o `null` si no existe.

#### Heredado de \{#inherited-from\}

[StorageReader](server.StorageReader.md).[getMetadata](server.StorageReader.md#getmetadata)

#### Definido en \{#defined-in\}

[server/storage.ts:75](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L75)

▸ **getMetadata**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

**`Deprecated`**

Esta función está obsoleta; utiliza `db.system.get(Id<"_storage">)` en su lugar.

Obtiene los metadatos de un archivo.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extiende `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | El [StorageId](../modules/server.md#storageid) del archivo. |

#### Returns \{#returns\}

`Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

* Un objeto [FileMetadata](../modules/server.md#filemetadata) si se encuentra; de lo contrario, `null`.

#### Heredado de \{#inherited-from\}

[StorageReader](server.StorageReader.md).[getMetadata](server.StorageReader.md#getmetadata)

#### Definido en \{#defined-in\}

[server/storage.ts:85](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L85)

***

### generateUploadUrl \{#generateuploadurl\}

▸ **generateUploadUrl**(): `Promise`&lt;`string`&gt;

Obtiene una URL temporal para subir un archivo al almacenamiento.

Cuando se realiza una solicitud POST a esta URL, el endpoint devuelve un objeto JSON que contiene un `Id<"_storage">` recién asignado.

La URL de POST acepta un encabezado HTTP Digest estándar opcional con una suma de comprobación sha256.

#### Devuelve \{#returns\}

`Promise`&lt;`string`&gt;

* Una URL que permite subir archivos mediante una solicitud HTTP POST.

#### Definido en \{#defined-in\}

[server/storage.ts:105](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L105)

***

### delete \{#delete\}

▸ **delete**(`storageId`): `Promise`&lt;`void`&gt;

Elimina un archivo del almacenamiento de Convex.

Una vez que se elimina un archivo, cualquier URL generada previamente por [getUrl](server.StorageReader.md#geturl) devolverá un código 404.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | El `Id<"_storage">` del archivo que se eliminará del almacenamiento de Convex. |

#### Devuelve \{#returns\}

`Promise`&lt;`void`&gt;

#### Definido en \{#defined-in\}

[server/storage.ts:113](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L113)

▸ **delete**&lt;`T`&gt;(`storageId`): `Promise`&lt;`void`&gt;

**`Deprecated`**

Pasar una cadena está en desuso; usa `storage.delete(Id<"_storage">)` en su lugar.

Elimina un archivo del almacenamiento de Convex.

Una vez que se elimina un archivo, cualquier URL generada previamente por [getUrl](server.StorageReader.md#geturl) devolverá 404.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extends `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | El [StorageId](../modules/server.md#storageid) del archivo que se va a eliminar del almacenamiento de Convex. |

#### Devuelve \{#returns\}

`Promise`&lt;`void`&gt;

#### Definido en \{#defined-in\}

[server/storage.ts:124](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L124)