---
id: "server.TableDefinition"
title: "Clase: TableDefinition<DocumentType, Indexes, SearchIndexes, VectorIndexes>"
custom_edit_url: null
---

[server](../modules/server.md).TableDefinition

La definición de una tabla dentro de un esquema.

Esta debe obtenerse usando [defineTable](../modules/server.md#definetable).

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DocumentType` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `any`, `any`&gt; = [`Validator`](../modules/values.md#validator)&lt;`any`, `any`, `any`&gt; |
| `Indexes` | extends [`GenericTableIndexes`](../modules/server.md#generictableindexes) = {} |
| `SearchIndexes` | extends [`GenericTableSearchIndexes`](../modules/server.md#generictablesearchindexes) = {} |
| `VectorIndexes` | extends [`GenericTableVectorIndexes`](../modules/server.md#generictablevectorindexes) = {} |

## Propiedades \{#properties\}

### validator \{#validator\}

• **validator**: `DocumentType`

#### Definido en \{#defined-in\}

[server/schema.ts:199](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L199)

## Métodos \{#methods\}

### indexes \{#indexes\}

▸ ** indexes**(): &#123; `indexDescriptor`: `string` ; `fields`: `string`[]  &#125;[]

Esta API es experimental: puede cambiar o desaparecer.

Devuelve los índices definidos en esta tabla.
Está pensada para casos de uso avanzados en los que se decide dinámicamente qué índice usar en una consulta.
Si crees que necesitas esto, por favor comenta en este issue en el repositorio de Convex JS en GitHub.
https://github.com/get-convex/convex-js/issues/49

#### Devuelve \{#returns\}

&#123; `indexDescriptor`: `string` ; `fields`: `string`[]  &#125;[]

#### Definido en \{#defined-in\}

[server/schema.ts:222](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L222)

***

### index \{#index\}

▸ **index**&lt;`IndexName`, `FirstFieldPath`, `RestFieldPaths`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, [`Expand`](../modules/server.md#expand)&lt;`Indexes` &amp; `Record`&lt;`IndexName`, [`FirstFieldPath`, ...RestFieldPaths[], `"_creationTime"`]&gt;&gt;, `SearchIndexes`, `VectorIndexes`&gt;

Define un índice para esta tabla.

Para obtener más información sobre los índices, consulta [Definición de índices](https://docs.convex.dev/using/indexes).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `IndexName` | extends `string` |
| `FirstFieldPath` | extends `any` |
| `RestFieldPaths` | extends `ExtractFieldPaths`&lt;`DocumentType`&gt;[] |

#### Parámetros \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `IndexName` | El nombre del índice. |
| `indexConfig` | `Object` | El objeto de configuración del índice. |
| `indexConfig.fields` | [`FirstFieldPath`, ...RestFieldPaths[]] | Los campos que se van a indexar, en orden. Debe especificarse al menos un campo. |
| `indexConfig.staged?` | `false` | Indica si el índice debe crearse como provisional (staged). Para tablas grandes, el backfill (rellenado) del índice puede ser lento. Marcar un índice como provisional te permite publicar el esquema y habilitar el índice más adelante. Si `staged` es `true`, el índice quedará en estado provisional y no se habilitará hasta que se elimine la marca de provisional. Los índices provisionales no bloquean la finalización del push. Los índices provisionales no pueden usarse en consultas. |

#### Devuelve \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, [`Expand`](../modules/server.md#expand)&lt;`Indexes` &amp; `Record`&lt;`IndexName`, [`FirstFieldPath`, ...RestFieldPaths[], `"_creationTime"`]&gt;&gt;, `SearchIndexes`, `VectorIndexes`&gt;

Una [TableDefinition](server.TableDefinition.md) con este índice incluido.

#### Definido en \{#defined-in\}

[server/schema.ts:235](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L235)

▸ **index**&lt;`IndexName`, `FirstFieldPath`, `RestFieldPaths`&gt;(`name`, `fields`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, [`Expand`](../modules/server.md#expand)&lt;`Indexes` &amp; `Record`&lt;`IndexName`, [`FirstFieldPath`, ...RestFieldPaths[], `"_creationTime"`]&gt;&gt;, `SearchIndexes`, `VectorIndexes`&gt;

Define un índice en esta tabla.

Para obtener más información sobre los índices, consulta [Defining Indexes](https://docs.convex.dev/using/indexes).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `IndexName` | extends `string` |
| `FirstFieldPath` | extends `any` |
| `RestFieldPaths` | extends `ExtractFieldPaths`&lt;`DocumentType`&gt;[] |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `name` | `IndexName` | El nombre del índice. |
| `fields` | [`FirstFieldPath`, ...RestFieldPaths[]] | Los campos que se van a indexar, en orden. Debe especificar al menos un campo. |

#### Devuelve \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, [`Expand`](../modules/server.md#expand)&lt;`Indexes` &amp; `Record`&lt;`IndexName`, [`FirstFieldPath`, ...RestFieldPaths[], `"_creationTime"`]&gt;&gt;, `SearchIndexes`, `VectorIndexes`&gt;

Una [TableDefinition](server.TableDefinition.md) que incluye este índice.

#### Definido en \{#defined-in\}

[server/schema.ts:268](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L268)

▸ **index**&lt;`IndexName`, `FirstFieldPath`, `RestFieldPaths`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

Define un índice en preparación en esta tabla.

Para tablas grandes, el rellenado (backfill) del índice puede ser lento. Preparar un índice te permite
hacer push del esquema y habilitar el índice más tarde.

Si `staged` es `true`, el índice quedará en preparación y no se habilitará
hasta que se elimine esta marca. Los índices en preparación no bloquean la
finalización del push. Los índices en preparación no se pueden usar en consultas.

Para obtener más información sobre los índices, consulta [Defining Indexes](https://docs.convex.dev/using/indexes).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `IndexName` | extends `string` |
| `FirstFieldPath` | extends `any` |
| `RestFieldPaths` | extends `ExtractFieldPaths`&lt;`DocumentType`&gt;[] |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `name` | `IndexName` | El nombre del índice. |
| `indexConfig` | `Object` | El objeto de configuración del índice. |
| `indexConfig.fields` | [`FirstFieldPath`, ...RestFieldPaths[]] | Los campos que se van a indexar, en orden. Debes especificar al menos un campo. |
| `indexConfig.staged` | `true` | Indica si el índice debe quedar en estado provisional. Para tablas grandes, el backfill del índice puede ser lento. Dejar un índice en estado provisional te permite hacer push del esquema y habilitar el índice más tarde. Si `staged` es `true`, el índice quedará en estado provisional y no se habilitará hasta que se quite la marca de provisional. Los índices en estado provisional no bloquean la finalización del push. Los índices en estado provisional no se pueden usar en consultas. |

#### Devuelve \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

Una [TableDefinition](server.TableDefinition.md) que incluye este índice.

#### Definido en \{#defined-in\}

[server/schema.ts:304](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L304)

***

### searchIndex \{#searchindex\}

▸ **searchIndex**&lt;`IndexName`, `SearchField`, `FilterFields`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, [`Expand`](../modules/server.md#expand)&lt;`SearchIndexes` &amp; `Record`&lt;`IndexName`, &#123; `searchField`: `SearchField` ; `filterFields`: `FilterFields`  &#125;&gt;&gt;, `VectorIndexes`&gt;

Define un índice de búsqueda en esta tabla.

Para obtener más información sobre los índices de búsqueda, consulta [Search](https://docs.convex.dev/text-search).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `IndexName` | extends `string` |
| `SearchField` | extends `any` |
| `FilterFields` | extends `any` = `never` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `name` | `IndexName` | El nombre del índice. |
| `indexConfig` | `Object` | El objeto de configuración del índice de búsqueda. |
| `indexConfig.searchField` | `SearchField` | El campo que se indexará para la búsqueda de texto completo. Debe ser un campo de tipo `string`. |
| `indexConfig.filterFields?` | `FilterFields`[] | Campos adicionales que se indexarán para un filtrado rápido al ejecutar consultas de búsqueda. |
| `indexConfig.staged?` | `false` | Indica si el índice debe estar en preparación (staged). En tablas grandes, el backfill del índice puede ser lento. Poner un índice en preparación permite enviar el esquema y habilitar el índice más tarde. Si `staged` es `true`, el índice quedará en preparación y no se habilitará hasta que se quite la marca de preparación. Los índices en preparación no bloquean la finalización de la operación de push. Los índices en preparación no se pueden usar en consultas. |

#### Devuelve \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, [`Expand`](../modules/server.md#expand)&lt;`SearchIndexes` &amp; `Record`&lt;`IndexName`, &#123; `searchField`: `SearchField` ; `filterFields`: `FilterFields`  &#125;&gt;&gt;, `VectorIndexes`&gt;

Una [`TableDefinition`](server.TableDefinition.md) con este índice de búsqueda incluido.

#### Definido en \{#defined-in\}

[server/schema.ts:357](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L357)

▸ **searchIndex**&lt;`IndexName`, `SearchField`, `FilterFields`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

Define un índice de búsqueda provisional en esta tabla.

Para tablas grandes, el relleno del índice puede ser lento. Dejar un índice en estado provisional te permite
hacer push del esquema y habilitar el índice más tarde.

Si `staged` es `true`, el índice quedará en estado provisional y no se habilitará
hasta que se elimine esta marca. Los índices provisionales no bloquean la
finalización del push. Los índices provisionales no se pueden usar en consultas.

Para obtener más información sobre los índices de búsqueda, consulta [Search](https://docs.convex.dev/text-search).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `IndexName` | extends `string` |
| `SearchField` | extends `any` |
| `FilterFields` | extends `any` = `never` |

#### Parámetros \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `IndexName` | El nombre del índice. |
| `indexConfig` | `Object` | El objeto de configuración del índice de búsqueda. |
| `indexConfig.searchField` | `SearchField` | El campo que se indexará para la búsqueda de texto completo. Debe ser un campo de tipo `string`. |
| `indexConfig.filterFields?` | `FilterFields`[] | Campos adicionales que se indexarán para filtrado rápido al ejecutar consultas de búsqueda. |
| `indexConfig.staged` | `true` | Indica si el índice debe crearse como provisional. En tablas grandes, el backfill del índice puede ser lento. Marcar un índice como provisional te permite hacer push del esquema y habilitar el índice más adelante. Si `staged` es `true`, el índice quedará en estado provisional y no se habilitará hasta que se quite la marca de provisional. Los índices provisionales no bloquean que el push se complete. Los índices provisionales no se pueden usar en consultas. |

#### Devuelve \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

Una [TableDefinition](server.TableDefinition.md) que incluye este índice de búsqueda.

#### Definido en \{#defined-in\}

[server/schema.ts:401](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L401)

***

### vectorIndex \{#vectorindex\}

▸ **vectorIndex**&lt;`IndexName`, `VectorField`, `FilterFields`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, [`Expand`](../modules/server.md#expand)&lt;`VectorIndexes` &amp; `Record`&lt;`IndexName`, &#123; `vectorField`: `VectorField` ; `dimensions`: `number` ; `filterFields`: `FilterFields`  &#125;&gt;&gt;&gt;

Define un índice vectorial en esta tabla.

Para más información sobre los índices vectoriales, consulta [Vector Search](https://docs.convex.dev/vector-search).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `IndexName` | extends `string` |
| `VectorField` | extends `any` |
| `FilterFields` | extends `any` = `never` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `name` | `IndexName` | El nombre del índice. |
| `indexConfig` | `Object` | El objeto de configuración del índice de vectores. |
| `indexConfig.vectorField` | `VectorField` | El campo que se va a indexar para la búsqueda vectorial. Debe ser un campo de tipo `v.array(v.float64())` (o una unión). |
| `indexConfig.dimensions` | `number` | La longitud de los vectores indexados. Debe estar entre 2 y 2048 inclusive. |
| `indexConfig.filterFields?` | `FilterFields`[] | Campos adicionales que se indexarán para filtrado rápido al ejecutar búsquedas vectoriales. |
| `indexConfig.staged?` | `false` | Indica si el índice debe estar en estado provisional. En tablas grandes, el rellenado del índice (backfill) puede ser lento. Marcar un índice como provisional te permite hacer push del esquema y habilitar el índice más tarde. Si `staged` es `true`, el índice quedará en estado provisional y no se habilitará hasta que se quite la marca de provisional. Los índices provisionales no bloquean la finalización del push y no se pueden usar en consultas. |

#### Devuelve \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, [`Expand`](../modules/server.md#expand)&lt;`VectorIndexes` &amp; `Record`&lt;`IndexName`, &#123; `vectorField`: `VectorField` ; `dimensions`: `number` ; `filterFields`: `FilterFields`  &#125;&gt;&gt;&gt;

Un [TableDefinition](server.TableDefinition.md) que incluye este índice vectorial.

#### Definido en \{#defined-in\}

[server/schema.ts:448](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L448)

▸ **vectorIndex**&lt;`IndexName`, `VectorField`, `FilterFields`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

Define un índice vectorial en estado provisional en esta tabla.

Para tablas grandes, el backfill del índice puede ser lento. Dejar un índice en estado provisional te permite
publicar el esquema y habilitar el índice más tarde.

Si `staged` es `true`, el índice quedará en estado provisional y no se habilitará
hasta que se elimine esta marca. Los índices en estado provisional no bloquean la
finalización del push. Los índices en estado provisional no se pueden usar en consultas.

Para aprender sobre índices vectoriales, consulta [Vector Search](https://docs.convex.dev/vector-search).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `IndexName` | extends `string` |
| `VectorField` | extends `any` |
| `FilterFields` | extends `any` = `never` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `name` | `IndexName` | El nombre del índice. |
| `indexConfig` | `Object` | El objeto de configuración del índice vectorial. |
| `indexConfig.vectorField` | `VectorField` | El campo que se va a indexar para la búsqueda vectorial. Debe ser un campo de tipo `v.array(v.float64())` (o una unión). |
| `indexConfig.dimensions` | `number` | La longitud de los vectores indexados. Debe estar entre 2 y 2048 inclusive. |
| `indexConfig.filterFields?` | `FilterFields`[] | Campos adicionales a indexar para filtrado rápido al ejecutar búsquedas vectoriales. |
| `indexConfig.staged` | `true` | Indica si el índice debe estar en estado provisional (staged). Para tablas grandes, el backfill del índice puede ser lento. Colocar un índice en estado provisional permite hacer push del esquema y habilitar el índice más adelante. Si `staged` es `true`, el índice estará en estado provisional y no se habilitará hasta que se quite la marca de provisional. Los índices en estado provisional no bloquean la finalización del push. Los índices en estado provisional no se pueden usar en consultas. |

#### Devuelve \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

Una [TableDefinition](server.TableDefinition.md) que incluye este índice vectorial.

#### Definido en \{#defined-in\}

[server/schema.ts:491](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L491)

***

### self \{#self\}

▸ `Protected` **self**(): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

Solución provisional para https://github.com/microsoft/TypeScript/issues/57035

#### Devuelve \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

#### Definido en \{#defined-in\}

[server/schema.ts:534](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L534)