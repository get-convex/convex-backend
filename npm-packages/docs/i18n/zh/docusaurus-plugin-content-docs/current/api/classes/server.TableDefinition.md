---
id: "server.TableDefinition"
title: "类：TableDefinition<DocumentType, Indexes, SearchIndexes, VectorIndexes>"
custom_edit_url: null
---

[server](../modules/server.md).TableDefinition

模式中某个表的定义。

应通过调用 [defineTable](../modules/server.md#definetable) 来生成。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DocumentType` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `any`, `any`&gt; = [`Validator`](../modules/values.md#validator)&lt;`any`, `any`, `any`&gt; |
| `Indexes` | extends [`GenericTableIndexes`](../modules/server.md#generictableindexes) = {} |
| `SearchIndexes` | extends [`GenericTableSearchIndexes`](../modules/server.md#generictablesearchindexes) = {} |
| `VectorIndexes` | extends [`GenericTableVectorIndexes`](../modules/server.md#generictablevectorindexes) = {} |

## 属性 \{#properties\}

### 验证器 \{#validator\}

• **validator**: `DocumentType`

#### 定义于 \{#defined-in\}

[server/schema.ts:199](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L199)

## 方法 \{#methods\}

### indexes \{#indexes\}

▸ ** indexes**(): &#123; `indexDescriptor`: `string` ; `fields`: `string`[]  &#125;[]

该 API 为实验性功能：其行为可能会变更或被移除。

返回在此表上定义的索引。
用于在高级场景下动态决定查询应使用哪个索引。
如果你觉得需要这个功能，请在 Convex JS GitHub 仓库中的这个 issue 里参与讨论。
https://github.com/get-convex/convex-js/issues/49

#### 返回 \{#returns\}

&#123; `indexDescriptor`: `string` ; `fields`: `string`[]  &#125;[]

#### 定义于 \{#defined-in\}

[server/schema.ts:222](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L222)

***

### index \{#index\}

▸ **index**&lt;`IndexName`, `FirstFieldPath`, `RestFieldPaths`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, [`Expand`](../modules/server.md#expand)&lt;`Indexes` &amp; `Record`&lt;`IndexName`, [`FirstFieldPath`, ...RestFieldPaths[], `"_creationTime"`]&gt;&gt;, `SearchIndexes`, `VectorIndexes`&gt;

在此表上定义索引。

要了解更多关于索引的内容，请参阅 [Defining Indexes](https://docs.convex.dev/using/indexes)。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `IndexName` | extends `string` |
| `FirstFieldPath` | extends `any` |
| `RestFieldPaths` | extends `ExtractFieldPaths`&lt;`DocumentType`&gt;[] |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `name` | `IndexName` | 索引的名称。 |
| `indexConfig` | `Object` | 索引配置对象。 |
| `indexConfig.fields` | [`FirstFieldPath`, ...RestFieldPaths[]] | 要按顺序建立索引的字段。必须至少指定一个字段。 |
| `indexConfig.staged?` | `false` | 索引是否应处于暂存（staged）状态。对于大型表，索引回填可能会很慢。将索引设为暂存可以让你先推送模式，然后稍后再启用索引。如果 `staged` 为 `true`，索引将处于暂存状态，在移除暂存标记之前不会被启用。暂存索引不会阻塞推送操作完成。暂存索引不能在查询中使用。 |

#### 返回值 \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, [`Expand`](../modules/server.md#expand)&lt;`Indexes` &amp; `Record`&lt;`IndexName`, [`FirstFieldPath`, ...RestFieldPaths[], `"_creationTime"`]&gt;&gt;, `SearchIndexes`, `VectorIndexes`&gt;

包含该索引的 [TableDefinition](server.TableDefinition.md)。

#### 定义于 \{#defined-in\}

[server/schema.ts:235](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L235)

▸ **index**&lt;`IndexName`, `FirstFieldPath`, `RestFieldPaths`&gt;(`name`, `fields`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, [`Expand`](../modules/server.md#expand)&lt;`Indexes` &amp; `Record`&lt;`IndexName`, [`FirstFieldPath`, ...RestFieldPaths[], `"_creationTime"`]&gt;&gt;, `SearchIndexes`, `VectorIndexes`&gt;

在此表上定义索引。

要了解索引的更多信息，请参阅 [定义索引](https://docs.convex.dev/using/indexes)。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `IndexName` | extends `string` |
| `FirstFieldPath` | extends `any` |
| `RestFieldPaths` | extends `ExtractFieldPaths`&lt;`DocumentType`&gt;[] |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `name` | `IndexName` | 索引的名称。 |
| `fields` | [`FirstFieldPath`, ...RestFieldPaths[]] | 要按顺序建立索引的字段。必须至少指定一个字段。 |

#### 返回值 \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, [`Expand`](../modules/server.md#expand)&lt;`Indexes` &amp; `Record`&lt;`IndexName`, [`FirstFieldPath`, ...RestFieldPaths[], `"_creationTime"`]&gt;&gt;, `SearchIndexes`, `VectorIndexes`&gt;

包含该索引的 [TableDefinition](server.TableDefinition.md)。

#### 定义于 \{#defined-in\}

[server/schema.ts:268](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L268)

▸ **index**&lt;`IndexName`, `FirstFieldPath`, `RestFieldPaths`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

在此表上定义一个暂存索引。

对于大型表，回填索引数据可能会很慢。通过先将索引暂存，你可以先推送模式，然后在之后再启用该索引。

如果 `staged` 为 `true`，索引将处于暂存状态并且不会启用，
直到移除该暂存标志。暂存索引不会阻塞推送
操作的完成。暂存索引不能在查询中使用。

要了解索引的更多信息，请参阅 [Defining Indexes](https://docs.convex.dev/using/indexes)。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `IndexName` | extends `string` |
| `FirstFieldPath` | extends `any` |
| `RestFieldPaths` | extends `ExtractFieldPaths`&lt;`DocumentType`&gt;[] |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `name` | `IndexName` | 索引名称。 |
| `indexConfig` | `Object` | 索引配置对象。 |
| `indexConfig.fields` | [`FirstFieldPath`, ...RestFieldPaths[]] | 按顺序要建立索引的字段。必须至少指定一个字段。 |
| `indexConfig.staged` | `true` | 索引是否应处于暂存（staged）状态。对于大型表，索引回填可能会很慢。将索引设为暂存状态可以让你先推送模式，然后稍后再启用索引。如果 `staged` 为 `true`，索引将被暂存，在移除暂存标志之前不会被启用。暂存索引不会阻塞推送完成，且不能在查询中使用。 |

#### 返回值 \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

包含该索引的 [TableDefinition](server.TableDefinition.md)。

#### 定义于 \{#defined-in\}

[server/schema.ts:304](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L304)

***

### searchIndex \{#searchindex\}

▸ **searchIndex**&lt;`IndexName`, `SearchField`, `FilterFields`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, [`Expand`](../modules/server.md#expand)&lt;`SearchIndexes` &amp; `Record`&lt;`IndexName`, &#123; `searchField`: `SearchField` ; `filterFields`: `FilterFields`  &#125;&gt;&gt;, `VectorIndexes`&gt;

在此表上定义一个搜索索引。

要进一步了解搜索索引，请参阅 [Search](https://docs.convex.dev/text-search)。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `IndexName` | extends `string` |
| `SearchField` | extends `any` |
| `FilterFields` | extends `any` = `never` |

#### 参数 \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `IndexName` | 索引名称。 |
| `indexConfig` | `Object` | 搜索索引的配置对象。 |
| `indexConfig.searchField` | `SearchField` | 用于全文搜索建索引的字段。该字段的类型必须为 `string`。 |
| `indexConfig.filterFields?` | `FilterFields`[] | 在运行搜索查询时，为加速过滤而额外建立索引的字段。 |
| `indexConfig.staged?` | `false` | 是否将索引置为 staged 状态。对于大型表，索引补建可能会比较慢。将索引设为 staged 可让你先推送模式（schema），再在之后启用索引。如果 `staged` 为 `true`，索引将处于 staged 状态，并且在移除 staged 标志之前不会被启用。staged 索引不会阻塞推送操作完成，但也不能在查询中使用。 |

#### 返回值 \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, [`Expand`](../modules/server.md#expand)&lt;`SearchIndexes` &amp; `Record`&lt;`IndexName`, &#123; `searchField`: `SearchField` ; `filterFields`: `FilterFields`  &#125;&gt;&gt;, `VectorIndexes`&gt;

一个包含该搜索索引的 [TableDefinition](server.TableDefinition.md)。

#### 定义位置 \{#defined-in\}

[server/schema.ts:357](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L357)

▸ **searchIndex**&lt;`IndexName`, `SearchField`, `FilterFields`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

在此表上定义一个暂存的搜索索引。

对于大型表，索引回填可能会很慢。将索引设为暂存状态可以让你
先推送模式，然后在之后再启用索引。

如果 `staged` 为 `true`，索引将处于暂存状态且不会被启用，
直到移除 staged 标志为止。暂存索引不会阻塞 push
的完成。暂存索引不能在查询中使用。

要了解有关搜索索引的更多信息，请参阅 [Search](https://docs.convex.dev/text-search)。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `IndexName` | extends `string` |
| `SearchField` | extends `any` |
| `FilterFields` | extends `any` = `never` |

#### 参数 \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `IndexName` | 索引名称。 |
| `indexConfig` | `Object` | 搜索索引配置对象。 |
| `indexConfig.searchField` | `SearchField` | 用于全文搜索建索引的字段。该字段必须是 `string` 类型。 |
| `indexConfig.filterFields?` | `FilterFields`[] | 在运行搜索查询时，为加速过滤而额外建立索引的字段。 |
| `indexConfig.staged` | `true` | 索引是否应处于预备（staged）状态。对于大型表，索引回填可能会很慢。将索引设为预备状态可以让你先推送模式，并在稍后再启用索引。如果 `staged` 为 `true`，索引将处于预备状态，在移除预备标记之前不会启用。预备索引不会阻塞推送完成，但也不能在查询中使用。 |

#### 返回值 \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

包含该搜索索引的 [TableDefinition](server.TableDefinition.md)。

#### 定义于 \{#defined-in\}

[server/schema.ts:401](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L401)

***

### vectorIndex \{#vectorindex\}

▸ **vectorIndex**&lt;`IndexName`, `VectorField`, `FilterFields`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, [`Expand`](../modules/server.md#expand)&lt;`VectorIndexes` &amp; `Record`&lt;`IndexName`, &#123; `vectorField`: `VectorField` ; `dimensions`: `number` ; `filterFields`: `FilterFields`  &#125;&gt;&gt;&gt;

在此表上定义一个向量索引。

要了解有关向量索引的更多信息，请参阅 [Vector Search](https://docs.convex.dev/vector-search)。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `IndexName` | extends `string` |
| `VectorField` | extends `any` |
| `FilterFields` | extends `any` = `never` |

#### 参数 \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `IndexName` | 索引的名称。 |
| `indexConfig` | `Object` | 向量索引配置对象。 |
| `indexConfig.vectorField` | `VectorField` | 用于向量搜索进行索引的字段。该字段的类型必须为 `v.array(v.float64())`（或联合类型）。 |
| `indexConfig.dimensions` | `number` | 被索引向量的长度。该值必须在 2 到 2048（含）之间。 |
| `indexConfig.filterFields?` | `FilterFields`[] | 在运行向量搜索时，为快速过滤而进行索引的附加字段。 |
| `indexConfig.staged?` | `false` | 索引是否应处于预备（staged）状态。对于大型表，索引回填可能会比较慢。将索引设为预备状态可以先推送模式，稍后再启用该索引。如果 `staged` 为 `true`，索引将处于预备状态，在移除预备标记之前不会启用。预备索引不会阻塞推送完成，但不能在查询中使用。 |

#### 返回值 \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, [`Expand`](../modules/server.md#expand)&lt;`VectorIndexes` &amp; `Record`&lt;`IndexName`, &#123; `vectorField`: `VectorField` ; `dimensions`: `number` ; `filterFields`: `FilterFields`  &#125;&gt;&gt;&gt;

包含该向量索引的 [TableDefinition](server.TableDefinition.md)。

#### 定义于 \{#defined-in\}

[server/schema.ts:448](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L448)

▸ **vectorIndex**&lt;`IndexName`, `VectorField`, `FilterFields`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

在此表上定义一个预备的向量索引。

对于大型表，索引回填可能会很慢。将索引设置为预备状态可以让你
先推送模式，再在稍后启用该索引。

如果 `staged` 为 `true`，索引将处于预备状态且不会被启用，
直到移除该预备标志。预备索引不会阻塞推送的完成。
预备索引不能在查询中使用。

要了解向量索引，参见 [Vector Search](https://docs.convex.dev/vector-search)。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `IndexName` | extends `string` |
| `VectorField` | extends `any` |
| `FilterFields` | extends `any` = `never` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `name` | `IndexName` | 索引名称。 |
| `indexConfig` | `Object` | 向量索引的配置对象。 |
| `indexConfig.vectorField` | `VectorField` | 用于向量搜索建立索引的字段。该字段的类型必须为 `v.array(v.float64())`（或包含此类型的 union）。 |
| `indexConfig.dimensions` | `number` | 被索引向量的长度。该值必须在 2 到 2048（含）之间。 |
| `indexConfig.filterFields?` | `FilterFields`[] | 在运行向量搜索时，为加速过滤而额外建立索引的字段。 |
| `indexConfig.staged` | `true` | 是否将索引设置为暂存（staged）状态。对于大型表，索引回填可能较慢。将索引暂存可以让你先推送模式，再在之后启用索引。如果 `staged` 为 `true`，索引将处于暂存状态，并且在移除暂存标记之前不会启用。暂存索引不会阻塞推送完成。暂存索引不能在查询中使用。 |

#### 返回值 \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

包含该向量索引的 [TableDefinition](server.TableDefinition.md)。

#### 定义于 \{#defined-in\}

[server/schema.ts:491](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L491)

***

### self \{#self\}

▸ `Protected` **self**(): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

用于规避此问题的变通方案：https://github.com/microsoft/TypeScript/issues/57035

#### 返回 \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

#### 定义于 \{#defined-in\}

[server/schema.ts:534](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L534)