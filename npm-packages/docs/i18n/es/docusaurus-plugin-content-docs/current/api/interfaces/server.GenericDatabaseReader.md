---
id: "server.GenericDatabaseReader"
title: "Interfaz: GenericDatabaseReader<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericDatabaseReader

Una interfaz para leer de la base de datos dentro de funciones de consulta de Convex.

Los dos puntos de entrada son:

* [get](server.GenericDatabaseReader.md#get), que recupera un único documento
  por su [GenericId](../modules/values.md#genericid).
  * [query](server.GenericDatabaseReader.md#query), que comienza a construir una consulta.

Si estás usando generación de código, usa el tipo `DatabaseReader` en
`convex/_generated/server.d.ts`, que tiene tipos específicos para tu modelo de datos.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extiende [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## Jerarquía \{#hierarchy\}

* `BaseDatabaseReader`&lt;`DataModel`&gt;

  ↳ **`GenericDatabaseReader`**

  ↳↳ [`GenericDatabaseWriter`](server.GenericDatabaseWriter.md)

## Propiedades \{#properties\}

### system \{#system\}

• **system**: `BaseDatabaseReader`&lt;[`SystemDataModel`](server.SystemDataModel.md)&gt;

Una interfaz para leer las tablas del sistema dentro de las funciones de consulta de Convex

Los dos puntos de entrada son:

* [get](server.GenericDatabaseReader.md#get), que recupera un único documento
  por su [GenericId](../modules/values.md#genericid).
  * [query](server.GenericDatabaseReader.md#query), que inicia la construcción de una consulta.

#### Definido en \{#defined-in\}

[server/database.ts:128](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L128)

## Métodos \{#methods\}

### get \{#get\}

▸ **get**&lt;`TableName`&gt;(`table`, `id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

Obtiene un único documento de la base de datos a partir de su [GenericId](../modules/values.md#genericid).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableName` | extends `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `table` | `TableName` | El nombre de la tabla de la que se obtendrá el documento. |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`NonUnion`&lt;`TableName`&gt;&gt; | El [GenericId](../modules/values.md#genericid) del documento que se obtendrá de la base de datos. |

#### Devuelve \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* El [GenericDocument](../modules/server.md#genericdocument) del documento con el [GenericId](../modules/values.md#genericid) especificado, o `null` si ya no existe.

#### Heredado de \{#inherited-from\}

BaseDatabaseReader.get

#### Definido en \{#defined-in\}

[server/database.ts:23](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L23)

▸ **get**&lt;`TableName`&gt;(`id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

Obtiene un solo documento de la base de datos por su [GenericId](../modules/values.md#genericid).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableName` | extends `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | El [GenericId](../modules/values.md#genericid) del documento que se va a recuperar de la base de datos. |

#### Devuelve \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* El [GenericDocument](../modules/server.md#genericdocument) del documento correspondiente al [GenericId](../modules/values.md#genericid) dado, o `null` si ya no existe.

#### Heredado de \{#inherited-from\}

BaseDatabaseReader.get

#### Definido en \{#defined-in\}

[server/database.ts:34](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L34)

***

### query \{#query\}

▸ **query**&lt;`TableName`&gt;(`tableName`): [`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

Inicia una consulta para la tabla indicada.

Las consultas no se ejecutan inmediatamente, así que llamar a este método y encadenar su
consulta no tiene ningún coste hasta que los resultados se utilizan realmente.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableName` | extiende `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `tableName` | `TableName` | El nombre de la tabla que se va a consultar. |

#### Devuelve \{#returns\}

[`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

* Un objeto [QueryInitializer](server.QueryInitializer.md) para comenzar a construir una consulta.

#### Heredado de \{#inherited-from\}

BaseDatabaseReader.query

#### Definido en \{#defined-in\}

[server/database.ts:47](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L47)

***

### normalizeId \{#normalizeid\}

▸ **normalizeId**&lt;`TableName`&gt;(`tableName`, `id`): `null` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;

Devuelve el ID en formato de cadena para un ID de una tabla dada, o `null` si el ID
pertenece a otra tabla o no es un ID válido.

Esta función acepta tanto el ID en formato de cadena como la representación `.toString()`
del formato heredado de ID basado en clases.

Esto no garantiza que el ID exista (es decir, `db.get(id)` puede devolver `null`).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableName` | extends `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `tableName` | `TableName` | El nombre de la tabla. |
| `id` | `string` | La cadena de Id. |

#### Devuelve \{#returns\}

`null` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;

#### Heredado de \{#inherited-from\}

BaseDatabaseReader.normalizeId

#### Definido en \{#defined-in\}

[server/database.ts:63](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L63)