---
title: "dataModel.d.ts"
sidebar_position: 1
description: "Tipos de TypeScript generados para el esquema de tu base de datos y sus documentos"
---

<Admonition type="caution" title="Este código se genera automáticamente">
  Estas exportaciones no están disponibles directamente en el paquete `convex`.

  En su lugar, debes ejecutar `npx convex dev` para crear
  `convex/_generated/dataModel.d.ts`.
</Admonition>

Tipos generados del modelo de datos.

## Tipos \{#types\}

### TableNames \{#tablenames\}

Ƭ **TableNames**: `string`

Los nombres de todas tus tablas de Convex.

***

### Doc \{#doc\}

Ƭ **Doc**`<TableName>`: `Object`

El tipo de un documento almacenado en Convex.

#### Parámetros de tipo \{#type-parameters\}

| Nombre      | Tipo                                | Descripción                                             |
| :---------- | :---------------------------------- | :------------------------------------------------------ |
| `TableName` | extends [`TableNames`](#tablenames) | Un tipo literal de cadena para el nombre de la tabla (por ejemplo, &quot;users&quot;). |

***

### Id \{#id\}

Un identificador para un documento en Convex.

Los documentos de Convex se identifican de manera única mediante su `Id`, que es accesible a través
del campo `_id`. Para obtener más información, consulta [Document IDs](/database/document-ids.mdx).

Los documentos se pueden cargar usando `db.get(tableName, id)` en funciones de consulta y mutación.

Los `Id` son solo cadenas en tiempo de ejecución, pero este tipo se puede usar para distinguirlos
de otras cadenas durante la comprobación de tipos.

Este es un alias de [`GenericId`](/api/modules/values#genericid) que está tipado
para tu modelo de datos.

#### Parámetros de tipo \{#type-parameters\}

| Nombre      | Tipo                                | Descripción                                                        |
| :---------- | :---------------------------------- | :----------------------------------------------------------------- |
| `TableName` | extends [`TableNames`](#tablenames) | Un tipo literal de cadena para el nombre de la tabla (por ejemplo, &quot;users&quot;). |

***

### DataModel \{#datamodel\}

Ƭ **DataModel**: `Object`

Un tipo que describe tu modelo de datos de Convex.

Este tipo incluye información sobre qué tablas contiene, el tipo de documentos
almacenados en esas tablas y los índices definidos en ellas.

Este tipo se usa para parametrizar métodos como
[`queryGeneric`](/api/modules/server#querygeneric) y
[`mutationGeneric`](/api/modules/server#mutationgeneric) para hacerlos seguros a nivel de tipos.