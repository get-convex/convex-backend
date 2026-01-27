---
id: "server.GenericDatabaseReaderWithTable"
title: "Interfaz: GenericDatabaseReaderWithTable<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericDatabaseReaderWithTable

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## Jerarquía \{#hierarchy\}

* `BaseDatabaseReaderWithTable`&lt;`DataModel`&gt;

  ↳ **`GenericDatabaseReaderWithTable`**

  ↳↳ [`GenericDatabaseWriterWithTable`](server.GenericDatabaseWriterWithTable.md)

## Propiedades \{#properties\}

### system \{#system\}

• **system**: `BaseDatabaseReaderWithTable`&lt;[`SystemDataModel`](server.SystemDataModel.md)&gt;

Una interfaz para leer las tablas del sistema dentro de las funciones de consulta de Convex

Los dos puntos de entrada son:

* [get](server.GenericDatabaseReader.md#get), que recupera un único documento
  por su [GenericId](../modules/values.md#genericid).
  * [query](server.GenericDatabaseReader.md#query), que comienza a construir una consulta.

#### Definido en \{#defined-in\}

[server/database.ts:144](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L144)

## Métodos \{#methods\}

### table \{#table\}

▸ **table**&lt;`TableName`&gt;(`tableName`): [`BaseTableReader`](server.BaseTableReader.md)&lt;`DataModel`, `TableName`&gt;

Restringe el ámbito de la base de datos a una tabla específica.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableName` | extends `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `tableName` | `TableName` |

#### Devuelve \{#returns\}

[`BaseTableReader`](server.BaseTableReader.md)&lt;`DataModel`, `TableName`&gt;

#### Heredado de \{#inherited-from\}

BaseDatabaseReaderWithTable.table

#### Definido en \{#defined-in\}

[server/database.ts:73](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L73)