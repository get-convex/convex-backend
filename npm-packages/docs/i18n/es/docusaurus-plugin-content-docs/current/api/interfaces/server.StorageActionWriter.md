---
id: "server.StorageActionWriter"
title: "Interfaz: StorageActionWriter"
custom_edit_url: null
---

[server](../modules/server.md).StorageActionWriter

Una interfaz para leer y escribir archivos en el almacenamiento dentro de acciones de Convex y de acciones HTTP.

## Jerarquía \{#hierarchy\}

* [`StorageWriter`](server.StorageWriter.md)

  ↳ **`StorageActionWriter`**

## Métodos \{#methods\}

### getUrl \{#geturl\}

▸ **getUrl**(`storageId`): `Promise`&lt;`null` | `string`&gt;

Obtiene la URL de un archivo en el almacenamiento mediante su `Id<"_storage">`.

La respuesta GET incluye una cabecera HTTP Digest estándar con una suma de comprobación SHA-256.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | El `Id<"_storage">` del archivo que se va a recuperar del almacenamiento de Convex. |

#### Devuelve \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* Una URL que permite obtener el archivo mediante una petición HTTP GET, o `null` si ya no existe.

#### Heredado de \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[getUrl](server.StorageWriter.md#geturl)

#### Definido en \{#defined-in\}

[server/storage.ts:51](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L51)

▸ **getUrl**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | `string`&gt;

**`Deprecated`**

Pasar un string está obsoleto; en su lugar, usa `storage.getUrl(Id<"_storage">)`.

Obtén la URL de un archivo en el almacenamiento a partir de su [StorageId](../modules/server.md#storageid).

La respuesta GET incluye una cabecera HTTP Digest estándar con una suma de verificación SHA-256.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extends `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | El [StorageId](../modules/server.md#storageid) del archivo que se va a recuperar desde el almacenamiento de Convex. |

#### Devuelve \{#returns\}

`Promise`&lt;`null` | `string`&gt;

* Una URL que recupera el archivo mediante una solicitud HTTP GET, o `null` si ya no existe.

#### Heredado de \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[getUrl](server.StorageWriter.md#geturl)

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

* Un objeto [FileMetadata](../modules/server.md#filemetadata) si se encuentra o `null` si no se encuentra.

#### Heredado de \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[getMetadata](server.StorageWriter.md#getmetadata)

#### Definido en \{#defined-in\}

[server/storage.ts:75](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L75)

▸ **getMetadata**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | [`FileMetadata`](../modules/server.md#filemetadata)&gt;

**`Deprecated`**

Esta función está obsoleta; usa `db.system.get(Id<"_storage">)` en su lugar.

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

#### Heredado de \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[getMetadata](server.StorageWriter.md#getmetadata)

#### Definido en \{#defined-in\}

[server/storage.ts:85](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L85)

***

### generateUploadUrl \{#generateuploadurl\}

▸ **generateUploadUrl**(): `Promise`&lt;`string`&gt;

Obtén una URL temporal para subir un archivo al almacenamiento.

Tras una solicitud POST a esta URL, el endpoint devolverá un objeto JSON que contiene un `Id<"_storage">` recién asignado.

La URL de POST acepta una cabecera estándar HTTP Digest opcional con una suma de comprobación sha256.

#### Devuelve \{#returns\}

`Promise`&lt;`string`&gt;

* Una URL que permite subir archivos mediante una solicitud HTTP POST.

#### Heredado de \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[generateUploadUrl](server.StorageWriter.md#generateuploadurl)

#### Definido en \{#defined-in\}

[server/storage.ts:105](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L105)

***

### delete \{#delete\}

▸ **delete**(`storageId`): `Promise`&lt;`void`&gt;

Elimina un archivo del almacenamiento de Convex.

Una vez que se elimina un archivo, cualquier URL generada previamente por [getUrl](server.StorageReader.md#geturl) devolverá un error 404.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; | El `Id<"_storage">` del archivo que se eliminará del almacenamiento de Convex. |

#### Devuelve \{#returns\}

`Promise`&lt;`void`&gt;

#### Heredado de \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[delete](server.StorageWriter.md#delete)

#### Definido en \{#defined-in\}

[server/storage.ts:113](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L113)

▸ **delete**&lt;`T`&gt;(`storageId`): `Promise`&lt;`void`&gt;

**`Deprecated`**

Pasar una cadena está obsoleto; usa `storage.delete(Id<"_storage">)` en su lugar.

Elimina un archivo del almacenamiento de Convex.

Una vez que se elimina un archivo, cualquier URL generada previamente por [getUrl](server.StorageReader.md#geturl) devolverá un 404.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extiende `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` | El [StorageId](../modules/server.md#storageid) del archivo que se eliminará del almacenamiento de Convex. |

#### Devuelve \{#returns\}

`Promise`&lt;`void`&gt;

#### Heredado de \{#inherited-from\}

[StorageWriter](server.StorageWriter.md).[delete](server.StorageWriter.md#delete)

#### Definido en \{#defined-in\}

[server/storage.ts:124](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L124)

***

### get \{#get\}

▸ **get**(`storageId`): `Promise`&lt;`null` | `Blob`&gt;

Devuelve un `Blob` que contiene el archivo asociado al `Id<"_storage">` proporcionado, o `null` si no existe ningún archivo.

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `storageId` | [`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt; |

#### Devuelve \{#returns\}

`Promise`&lt;`null` | `Blob`&gt;

#### Definido en \{#defined-in\}

[server/storage.ts:138](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L138)

▸ **get**&lt;`T`&gt;(`storageId`): `Promise`&lt;`null` | `Blob`&gt;

**`Deprecated`**

Pasar una cadena está obsoleto; usa `storage.get(Id<"_storage">)` en su lugar.

Devuelve un `Blob` que contiene el archivo asociado con el [StorageId](../modules/server.md#storageid) proporcionado, o `null` si no hay ningún archivo.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extends `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `storageId` | `T` extends &#123; `__tableName`: `any`  &#125; ? `never` : `T` |

#### Devuelve \{#returns\}

`Promise`&lt;`null` | `Blob`&gt;

#### Definido en \{#defined-in\}

[server/storage.ts:145](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L145)

***

### store \{#store\}

▸ **store**(`blob`, `options?`): `Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt;&gt;

Almacena el archivo contenido en el Blob.

Si se indica, se verificará que la suma de comprobación SHA-256 coincida con el contenido del archivo.

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `blob` | `Blob` |
| `options?` | `Object` |
| `options.sha256?` | `string` |

#### Devuelve \{#returns\}

`Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_storage"`&gt;&gt;

#### Definido en \{#defined-in\}

[server/storage.ts:153](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L153)