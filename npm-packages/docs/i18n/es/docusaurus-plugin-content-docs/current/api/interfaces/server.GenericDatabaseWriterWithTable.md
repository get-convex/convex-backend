---
id: "server.GenericDatabaseWriterWithTable"
title: "Interfaz: GenericDatabaseWriterWithTable<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericDatabaseWriterWithTable

Interfaz para leer y escribir en la base de datos dentro de funciones de mutación de Convex.

Convex garantiza que todas las escrituras dentro de una única mutación se
ejecuten de forma atómica, por lo que nunca tienes que preocuparte por escrituras
parciales que dejen tus datos en un estado inconsistente. Consulta [la guía de Convex](https://docs.convex.dev/understanding/convex-fundamentals/functions#atomicity-and-optimistic-concurrency-control)
para conocer las garantías que Convex ofrece a tus funciones.

Si estás usando generación de código, usa el tipo `DatabaseReader` en
`convex/_generated/server.d.ts`, que está tipado para tu modelo de datos.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extiende [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## Jerarquía \{#hierarchy\}

* [`GenericDatabaseReaderWithTable`](server.GenericDatabaseReaderWithTable.md)&lt;`DataModel`&gt;

  ↳ **`GenericDatabaseWriterWithTable`**

## Propiedades \{#properties\}

### system \{#system\}

• **system**: `BaseDatabaseReaderWithTable`&lt;[`SystemDataModel`](server.SystemDataModel.md)&gt;

Una interfaz para leer las tablas del sistema dentro de las funciones de consulta de Convex

Los dos puntos de entrada son:

* [get](server.GenericDatabaseReader.md#get), que recupera un único documento
  por su [GenericId](../modules/values.md#genericid).
  * [query](server.GenericDatabaseReader.md#query), que comienza a construir una consulta.

#### Heredado de \{#inherited-from\}

[GenericDatabaseReaderWithTable](server.GenericDatabaseReaderWithTable.md).[system](server.GenericDatabaseReaderWithTable.md#system)

#### Definido en \{#defined-in\}

[server/database.ts:144](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L144)

## Métodos \{#methods\}

### table \{#table\}

▸ **table**&lt;`TableName`&gt;(`tableName`): [`BaseTableWriter`](server.BaseTableWriter.md)&lt;`DataModel`, `TableName`&gt;

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

[`BaseTableWriter`](server.BaseTableWriter.md)&lt;`DataModel`, `TableName`&gt;

#### Sobrescrituras \{#overrides\}

[GenericDatabaseReaderWithTable](server.GenericDatabaseReaderWithTable.md).[table](server.GenericDatabaseReaderWithTable.md#table)

#### Definido en \{#defined-in\}

[server/database.ts:274](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L274)