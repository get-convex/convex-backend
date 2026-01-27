---
id: "server.BaseTableWriter"
title: "Interfaz: BaseTableWriter<DataModel, TableName>"
custom_edit_url: null
---

[server](../modules/server.md).BaseTableWriter

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extiende [`GenericDataModel`](../modules/server.md#genericdatamodel) |
| `TableName` | extiende [`TableNamesInDataModel`](../modules/server.md#tablenamesindatamodel)&lt;`DataModel`&gt; |

## Jerarquía \{#hierarchy\}

* [`BaseTableReader`](server.BaseTableReader.md)&lt;`DataModel`, `TableName`&gt;

  ↳ **`BaseTableWriter`**

## Métodos \{#methods\}

### get \{#get\}

▸ **get**(`id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

Obtiene un único documento de la tabla mediante su [GenericId](../modules/values.md#genericid).

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | El [GenericId](../modules/values.md#genericid) del documento que se va a obtener de la base de datos. |

#### Devuelve \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* El [GenericDocument](../modules/server.md#genericdocument) del documento con el [GenericId](../modules/values.md#genericid) especificado, o `null` si ya no existe.

#### Heredado de \{#inherited-from\}

[BaseTableReader](server.BaseTableReader.md).[get](server.BaseTableReader.md#get)

#### Definido en \{#defined-in\}

[server/database.ts:88](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L88)

***

### query \{#query\}

▸ **query**(): [`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

Inicia una consulta para la tabla.

Las consultas no se ejecutan de inmediato, por lo que invocar este método y ampliar la consulta resultante no tiene ningún coste hasta que los resultados se usan realmente.

#### Returns \{#returns\}

[`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

* Un objeto [QueryInitializer](server.QueryInitializer.md) para comenzar a construir una consulta.

#### Heredado de \{#inherited-from\}

[BaseTableReader](server.BaseTableReader.md).[query](server.BaseTableReader.md#query)

#### Definido en \{#defined-in\}

[server/database.ts:100](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L100)

***

### insert \{#insert\}

▸ **insert**(`value`): `Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;&gt;

Inserta un nuevo documento en la tabla.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `value` | [`WithoutSystemFields`](../modules/server.md#withoutsystemfields)&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | El [Valor](../modules/values.md#value) que se va a insertar en la tabla especificada. |

#### Devuelve \{#returns\}

`Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt;&gt;

* [GenericId](../modules/values.md#genericid) del nuevo documento.

#### Definido en \{#defined-in\}

[server/database.ts:289](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L289)

***

### patch \{#patch\}

▸ **patch**(`id`, `value`): `Promise`&lt;`void`&gt;

Actualiza parcialmente un documento existente, combinándolo con el documento parcial proporcionado mediante una fusión superficial.

Se añaden nuevos campos. Los campos existentes se sobrescriben. Los campos cuyo valor se establece en
`undefined` se eliminan.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | El [GenericId](../modules/values.md#genericid) del documento que se va a actualizar mediante un *patch*. |
| `value` | `PatchValue`&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | El [GenericDocument](../modules/server.md#genericdocument) parcial que se va a fusionar en el documento especificado. Si este nuevo valor especifica campos de sistema como `_id`, deben coincidir con los valores actuales de esos campos en el documento. |

#### Devuelve \{#returns\}

`Promise`&lt;`void`&gt;

#### Definido en \{#defined-in\}

[server/database.ts:304](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L304)

***

### replace \{#replace\}

▸ **replace**(`id`, `value`): `Promise`&lt;`void`&gt;

Reemplaza el valor de un documento existente, sobrescribiendo su valor anterior.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | El [GenericId](../modules/values.md#genericid) del documento que se va a reemplazar. |
| `value` | [`WithOptionalSystemFields`](../modules/server.md#withoptionalsystemfields)&lt;[`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt; | El nuevo [GenericDocument](../modules/server.md#genericdocument) para el documento. Este valor puede omitir los campos del sistema, y la base de datos los completará. |

#### Devuelve \{#returns\}

`Promise`&lt;`void`&gt;

#### Definido en \{#defined-in\}

[server/database.ts:316](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L316)

***

### delete \{#delete\}

▸ **delete**(`id`): `Promise`&lt;`void`&gt;

Elimina un documento existente.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | El [GenericId](../modules/values.md#genericid) del documento que se eliminará. |

#### Devuelve \{#returns\}

`Promise`&lt;`void`&gt;

#### Definido en \{#defined-in\}

[server/database.ts:326](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L326)