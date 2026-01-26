---
id: "server.GenericDatabaseWriter"
title: "Interfaz: GenericDatabaseWriter<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericDatabaseWriter

Una interfaz para leer y escribir en la base de datos dentro de las funciones
de mutación de Convex.

Convex garantiza que todas las escrituras dentro de una sola mutación se
ejecutan de forma atómica, por lo que nunca tienes que preocuparte de que
escrituras parciales dejen tus datos en un estado inconsistente. Consulta
[la guía de Convex](https://docs.convex.dev/understanding/convex-fundamentals/functions#atomicity-and-optimistic-concurrency-control)
para ver las garantías que Convex proporciona a tus funciones.

Si utilizas la generación de código, usa el tipo `DatabaseReader` en
`convex/_generated/server.d.ts`, que está tipado para tu modelo de datos.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## Jerarquía \{#hierarchy\}

* [`GenericDatabaseReader`](server.GenericDatabaseReader.md)&lt;`DataModel`&gt;

  ↳ **`GenericDatabaseWriter`**

## Propiedades \{#properties\}

### system \{#system\}

• **system**: `BaseDatabaseReader`&lt;[`SystemDataModel`](server.SystemDataModel.md)&gt;

Una interfaz para leer las tablas del sistema dentro de funciones de consulta de Convex

Los dos puntos de entrada son:

* [get](server.GenericDatabaseReader.md#get), que obtiene un solo documento
  por su [GenericId](../modules/values.md#genericid).
  * [query](server.GenericDatabaseReader.md#query), que comienza a construir una consulta.

#### Heredado de \{#inherited-from\}

[GenericDatabaseReader](server.GenericDatabaseReader.md).[system](server.GenericDatabaseReader.md#system)

#### Definido en \{#defined-in\}

[server/database.ts:128](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L128)

## Métodos \{#methods\}

### get \{#get\}

▸ **get**&lt;`TableName`&gt;(`table`, `id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

Recupera un único documento de la base de datos mediante su [GenericId](../modules/values.md#genericid).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableName` | extiende `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `table` | `TableName` | El nombre de la tabla de la que se obtendrá el documento. |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`NonUnion`&lt;`TableName`&gt;&gt; | El [GenericId](../modules/values.md#genericid) del documento que se va a obtener de la base de datos. |

#### Devuelve \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* El [GenericDocument](../modules/server.md#genericdocument) del documento con el [GenericId](../modules/values.md#genericid) indicado, o `null` si ya no existe.

#### Heredado de \{#inherited-from\}

[GenericDatabaseReader](server.GenericDatabaseReader.md).[get](server.GenericDatabaseReader.md#get)

#### Definido en \{#defined-in\}

[server/database.ts:23](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L23)

▸ **get**&lt;`TableName`&gt;(`id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

Obtiene un único documento de la base de datos por su [GenericId](../modules/values.md#genericid).

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

[GenericDatabaseReader](server.GenericDatabaseReader.md).[get](server.GenericDatabaseReader.md#get)

#### Definido en \{#defined-in\}

[server/database.ts:34](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L34)

***

### query \{#query\}

▸ **query**&lt;`TableName`&gt;(`tableName`): [`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

Inicia una consulta para la tabla indicada.

Las consultas no se ejecutan inmediatamente, por lo que llamar a este método y extender su
consulta no tiene coste hasta que los resultados se usan realmente.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableName` | extends `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `tableName` | `TableName` | El nombre de la tabla a consultar. |

#### Devuelve \{#returns\}

[`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

* Un objeto [QueryInitializer](server.QueryInitializer.md) para comenzar a construir una consulta.

#### Heredado de \{#inherited-from\}

[GenericDatabaseReader](server.GenericDatabaseReader.md).[query](server.GenericDatabaseReader.md#query)

#### Definido en \{#defined-in\}

[server/database.ts:47](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L47)

***

### normalizeId \{#normalizeid\}

▸ **normalizeId**&lt;`TableName`&gt;(`tableName`, `id`): `null` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;

Devuelve la ID en formato de cadena para una tabla dada, o null si la ID
pertenece a otra tabla o no es una ID válida.

Acepta tanto la ID en formato de cadena como la representación `.toString()`
del formato de ID heredado basado en clases.

Esto no garantiza que la ID exista (es decir, `db.get(id)` puede devolver `null`).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableName` | extends `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `tableName` | `TableName` | El nombre de la tabla. |
| `id` | `string` | La cadena de ID. |

#### Devuelve \{#returns\}

`null` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;

#### Heredado de \{#inherited-from\}

[GenericDatabaseReader](server.GenericDatabaseReader.md).[normalizeId](server.GenericDatabaseReader.md#normalizeid)

#### Definido en \{#defined-in\}

[server/database.ts:63](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L63)

***

### insert \{#insert\}

▸ **insert**&lt;`TableName`&gt;(`table`, `value`): `Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;&gt;

Insertar un nuevo documento en una tabla.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableName` | extiende `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `table` | `TableName` | El nombre de la tabla en la que insertar un nuevo documento. |
| `value` | [`WithoutSystemFields`](../modules/server.md#withoutsystemfields)&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | El [Valor](../modules/values.md#value) que se va a insertar en la tabla especificada. |

#### Devuelve \{#returns\}

`Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;&gt;

* [GenericId](../modules/values.md#genericid) del nuevo documento.

#### Definido en \{#defined-in\}

[server/database.ts:170](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L170)

***

### patch \{#patch\}

▸ **patch**&lt;`TableName`&gt;(`table`, `id`, `value`): `Promise`&lt;`void`&gt;

Modifica parcialmente un documento existente, combinándolo mediante una fusión superficial con el
documento parcial proporcionado.

Se añaden campos nuevos. Los campos existentes se sobrescriben. Los campos establecidos en
`undefined` se eliminan.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableName` | extends `string` |

#### Parámetros \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `table` | `TableName` | El nombre de la tabla en la que está el documento. |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`NonUnion`&lt;`TableName`&gt;&gt; | El [GenericId](../modules/values.md#genericid) del documento que se va a actualizar con `patch`. |
| `value` | `PatchValue`&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | El [GenericDocument](../modules/server.md#genericdocument) parcial que se va a fusionar en el documento especificado. Si este nuevo valor especifica campos de sistema como `_id`, deben coincidir con los valores de los campos existentes del documento. |

#### Devuelve \{#returns\}

`Promise`&lt;`void`&gt;

#### Definido en \{#defined-in\}

[server/database.ts:187](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L187)

▸ **patch**&lt;`TableName`&gt;(`id`, `value`): `Promise`&lt;`void`&gt;

Actualiza parcialmente un documento existente, combinándolo de forma superficial con el
documento parcial proporcionado.

Se agregan nuevos campos. Los campos existentes se sobrescriben. Los campos configurados como
`undefined` se eliminan.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableName` | extiende `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | El [GenericId](../modules/values.md#genericid) del documento que se va a modificar. |
| `value` | `PatchValue`&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | El [GenericDocument](../modules/server.md#genericdocument) parcial que se va a fusionar con el documento especificado. Si este nuevo valor especifica campos de sistema como `_id`, deben coincidir con los valores actuales de los campos del documento. |

#### Devuelve \{#returns\}

`Promise`&lt;`void`&gt;

#### Definido en \{#defined-in\}

[server/database.ts:204](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L204)

***

### replace \{#replace\}

▸ **replace**&lt;`TableName`&gt;(`table`, `id`, `value`): `Promise`&lt;`void`&gt;

Reemplaza el valor de un documento existente, sobrescribiendo su valor anterior.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableName` | extiende `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `table` | `TableName` | El nombre de la tabla en la que se encuentra el documento. |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`NonUnion`&lt;`TableName`&gt;&gt; | El [GenericId](../modules/values.md#genericid) del documento que se va a reemplazar. |
| `value` | [`WithOptionalSystemFields`](../modules/server.md#withoptionalsystemfields)&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | El nuevo [GenericDocument](../modules/server.md#genericdocument) para el documento. Este valor puede omitir los campos de sistema y la base de datos los completará. |

#### Devuelve \{#returns\}

`Promise`&lt;`void`&gt;

#### Definido en \{#defined-in\}

[server/database.ts:217](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L217)

▸ **replace**&lt;`TableName`&gt;(`id`, `value`): `Promise`&lt;`void`&gt;

Reemplaza el valor de un documento existente, sobrescribiendo su valor anterior.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableName` | extiende `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | El [GenericId](../modules/values.md#genericid) del documento que se reemplazará. |
| `value` | [`WithOptionalSystemFields`](../modules/server.md#withoptionalsystemfields)&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | El nuevo [GenericDocument](../modules/server.md#genericdocument) del documento. Este valor puede omitir los campos del sistema y la base de datos los completará. |

#### Devuelve \{#returns\}

`Promise`&lt;`void`&gt;

#### Definido en \{#defined-in\}

[server/database.ts:230](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L230)

***

### delete \{#delete\}

▸ **delete**&lt;`TableName`&gt;(`table`, `id`): `Promise`&lt;`void`&gt;

Elimina un documento existente.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableName` | extends `string` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `table` | `TableName` | El nombre de la tabla en la que se encuentra el documento. |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`NonUnion`&lt;`TableName`&gt;&gt; | El [GenericId](../modules/values.md#genericid) del documento que se va a eliminar. |

#### Devuelve \{#returns\}

`Promise`&lt;`void`&gt;

#### Definido en \{#defined-in\}

[server/database.ts:241](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L241)

▸ **delete**(`id`): `Promise`&lt;`void`&gt;

Elimina un documento existente.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;[`TableNamesInDataModel`](../modules/server.md#tablenamesindatamodel)&lt;`DataModel`&gt;&gt; | El [GenericId](../modules/values.md#genericid) del documento que se va a eliminar. |

#### Devuelve \{#returns\}

`Promise`&lt;`void`&gt;

#### Definido en \{#defined-in\}

[server/database.ts:251](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L251)