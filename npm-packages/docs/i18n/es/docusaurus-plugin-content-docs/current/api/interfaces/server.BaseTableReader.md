---
id: "server.BaseTableReader"
title: "Interfaz: BaseTableReader<DataModel, TableName>"
custom_edit_url: null
---

[server](../modules/server.md).BaseTableReader

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](../modules/server.md#genericdatamodel) |
| `TableName` | extends [`TableNamesInDataModel`](../modules/server.md#tablenamesindatamodel)&lt;`DataModel`&gt; |

## Jerarquía \{#hierarchy\}

* **`BaseTableReader`**

  ↳ [`BaseTableWriter`](server.BaseTableWriter.md)

## Métodos \{#methods\}

### get \{#get\}

▸ **get**(`id`): `Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

Recupera un único documento de la tabla mediante su [GenericId](../modules/values.md#genericid).

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`TableName`&gt; | El [GenericId](../modules/values.md#genericid) del documento que se va a recuperar de la base de datos. |

#### Returns \{#returns\}

`Promise`&lt;`null` | [`DocumentByName`](../modules/server.md#documentbyname)&lt;`DataModel`, `TableName`&gt;&gt;

* El [GenericDocument](../modules/server.md#genericdocument) del documento con el [GenericId](../modules/values.md#genericid) dado, o `null` si el documento ya no existe.

#### Definido en \{#defined-in\}

[server/database.ts:88](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L88)

***

### query \{#query\}

▸ **query**(): [`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

Inicia una consulta sobre la tabla.

Las consultas no se ejecutan inmediatamente, por lo que invocar este método y ampliar la
consulta no tiene coste hasta que los resultados se utilizan realmente.

#### Returns \{#returns\}

[`QueryInitializer`](server.QueryInitializer.md)&lt;[`NamedTableInfo`](../modules/server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt;

* Un objeto [QueryInitializer](server.QueryInitializer.md) para iniciar la construcción de una consulta.

#### Definido en \{#defined-in\}

[server/database.ts:100](https://github.com/get-convex/convex-js/blob/main/src/server/database.ts#L100)