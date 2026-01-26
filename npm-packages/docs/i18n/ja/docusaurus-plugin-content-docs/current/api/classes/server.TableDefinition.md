---
id: "server.TableDefinition"
title: "クラス: TableDefinition<DocumentType, Indexes, SearchIndexes, VectorIndexes>"
custom_edit_url: null
---

[server](../modules/server.md).TableDefinition

スキーマ内のテーブルの定義です。

これは [defineTable](../modules/server.md#definetable) を使って定義します。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `DocumentType` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `any`, `any`&gt; = [`Validator`](../modules/values.md#validator)&lt;`any`, `any`, `any`&gt; |
| `Indexes` | extends [`GenericTableIndexes`](../modules/server.md#generictableindexes) = {} |
| `SearchIndexes` | extends [`GenericTableSearchIndexes`](../modules/server.md#generictablesearchindexes) = {} |
| `VectorIndexes` | extends [`GenericTableVectorIndexes`](../modules/server.md#generictablevectorindexes) = {} |

## プロパティ \{#properties\}

### validator \{#validator\}

• **validator**: `DocumentType`

#### 定義元 \{#defined-in\}

[server/schema.ts:199](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L199)

## メソッド \{#methods\}

### indexes \{#indexes\}

▸ ** indexes**(): &#123; `indexDescriptor`: `string` ; `fields`: `string`[]  &#125;[]

この API は実験的なものであり、変更されたり削除されたりする可能性があります。

このテーブルで定義されているインデックスを返します。
クエリに対してどのインデックスを使用するかを動的に決定するといった高度なユースケースを想定しています。
これが必要だと思う場合は、Convex JS の GitHub リポジトリ内のこの issue で議論に参加してください。
https://github.com/get-convex/convex-js/issues/49

#### 戻り値 \{#returns\}

&#123; `indexDescriptor`: `string` ; `fields`: `string`[]  &#125;[]

#### 定義元 \{#defined-in\}

[server/schema.ts:222](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L222)

***

### index \{#index\}

▸ **index**&lt;`IndexName`, `FirstFieldPath`, `RestFieldPaths`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, [`Expand`](../modules/server.md#expand)&lt;`Indexes` &amp; `Record`&lt;`IndexName`, [`FirstFieldPath`, ...RestFieldPaths[], `"_creationTime"`]&gt;&gt;, `SearchIndexes`, `VectorIndexes`&gt;

このテーブルにインデックスを定義します。

インデックスの定義方法については、[Defining Indexes](https://docs.convex.dev/using/indexes) を参照してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `IndexName` | extends `string` |
| `FirstFieldPath` | extends `any` |
| `RestFieldPaths` | extends `ExtractFieldPaths`&lt;`DocumentType`&gt;[] |

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `IndexName` | インデックスの名前。 |
| `indexConfig` | `Object` | インデックス設定オブジェクト。 |
| `indexConfig.fields` | [`FirstFieldPath`, ...RestFieldPaths[]] | インデックスを作成するフィールド。指定した順序でインデックスされます。少なくとも 1 つのフィールドを指定する必要があります。 |
| `indexConfig.staged?` | `false` | インデックスをステージングするかどうか。大きなテーブルでは、インデックスのバックフィルに時間がかかる場合があります。インデックスをステージングすると、まずスキーマをプッシュし、その後でインデックスを有効化できます。`staged` が `true` の場合、インデックスはステージングされ、ステージングフラグが削除されるまで有効になりません。ステージングされたインデックスはプッシュの完了を妨げません。ステージングされたインデックスはクエリで使用できません。 |

#### Returns \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, [`Expand`](../modules/server.md#expand)&lt;`Indexes` &amp; `Record`&lt;`IndexName`, [`FirstFieldPath`, ...RestFieldPaths[], `"_creationTime"`]&gt;&gt;, `SearchIndexes`, `VectorIndexes`&gt;

このインデックスを含む [TableDefinition](server.TableDefinition.md) を返します。

#### 定義場所 \{#defined-in\}

[server/schema.ts:235](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L235)

▸ **index**&lt;`IndexName`, `FirstFieldPath`, `RestFieldPaths`&gt;(`name`, `fields`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, [`Expand`](../modules/server.md#expand)&lt;`Indexes` &amp; `Record`&lt;`IndexName`, [`FirstFieldPath`, ...RestFieldPaths[], `"_creationTime"`]&gt;&gt;, `SearchIndexes`, `VectorIndexes`&gt;

このテーブルのインデックスを定義します。

インデックスの詳細については、[Defining Indexes](https://docs.convex.dev/using/indexes) を参照してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `IndexName` | extends `string` |
| `FirstFieldPath` | extends `any` |
| `RestFieldPaths` | extends `ExtractFieldPaths`&lt;`DocumentType`&gt;[] |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `IndexName` | インデックスの名前。 |
| `fields` | [`FirstFieldPath`, ...RestFieldPaths[]] | インデックスするフィールドを順番に指定します。少なくとも 1 つのフィールドを指定する必要があります。 |

#### 戻り値 \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, [`Expand`](../modules/server.md#expand)&lt;`Indexes` &amp; `Record`&lt;`IndexName`, [`FirstFieldPath`, ...RestFieldPaths[], `"_creationTime"`]&gt;&gt;, `SearchIndexes`, `VectorIndexes`&gt;

このインデックスを含む [TableDefinition](server.TableDefinition.md) を返します。

#### 定義箇所 \{#defined-in\}

[server/schema.ts:268](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L268)

▸ **index**&lt;`IndexName`, `FirstFieldPath`, `RestFieldPaths`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

このテーブルにステージドインデックスを定義します。

大きなテーブルでは、インデックスのバックフィル処理に時間がかかる場合があります。インデックスをステージすると、
先にスキーマをプッシュしておき、後からインデックスを有効化できます。

`staged` が `true` の場合、インデックスはステージされ、有効にはなりません。
ステージドインデックスはプッシュの完了をブロックしません。
ステージドインデックスはクエリで使用できません。

インデックスについて詳しくは、[Defining Indexes](https://docs.convex.dev/using/indexes) を参照してください。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `IndexName` | extends `string` |
| `FirstFieldPath` | extends `any` |
| `RestFieldPaths` | extends `ExtractFieldPaths`&lt;`DocumentType`&gt;[] |

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `IndexName` | インデックスの名前。 |
| `indexConfig` | `Object` | インデックス構成オブジェクト。 |
| `indexConfig.fields` | [`FirstFieldPath`, ...RestFieldPaths[]] | インデックス対象とするフィールド（指定した順序で）。少なくとも 1 つのフィールドを指定する必要があります。 |
| `indexConfig.staged` | `true` | インデックスをステージングするかどうか。大きなテーブルでは、インデックスのバックフィル処理に時間がかかる場合があります。インデックスをステージングすると、スキーマをプッシュしておき、インデックスは後から有効化できます。`staged` が `true` の場合、インデックスはステージングされ、ステージングフラグが削除されるまで有効になりません。ステージングされたインデックスはプッシュ処理の完了をブロックせず、クエリで使用することはできません。 |

#### 戻り値 \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

このインデックスを含む [TableDefinition](server.TableDefinition.md) を返します。

#### 定義元 \{#defined-in\}

[server/schema.ts:304](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L304)

***

### searchIndex \{#searchindex\}

▸ **searchIndex**&lt;`IndexName`, `SearchField`, `FilterFields`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, [`Expand`](../modules/server.md#expand)&lt;`SearchIndexes` &amp; `Record`&lt;`IndexName`, &#123; `searchField`: `SearchField` ; `filterFields`: `FilterFields`  &#125;&gt;&gt;, `VectorIndexes`&gt;

このテーブルに検索インデックスを定義します。

検索インデックスについて詳しくは、[Search](https://docs.convex.dev/text-search) を参照してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `IndexName` | extends `string` |
| `SearchField` | extends `any` |
| `FilterFields` | extends `any` = `never` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `IndexName` | インデックス名。 |
| `indexConfig` | `Object` | 検索インデックスの設定オブジェクト。 |
| `indexConfig.searchField` | `SearchField` | 全文検索のためにインデックスするフィールド。このフィールドは `string` 型である必要があります。 |
| `indexConfig.filterFields?` | `FilterFields`[] | 検索クエリを実行する際の高速なフィルタリングのためにインデックスする追加フィールド。 |
| `indexConfig.staged?` | `false` | インデックスをステージングするかどうか。大きなテーブルでは、インデックスのバックフィル処理に時間がかかる場合があります。インデックスをステージングすると、スキーマを push しておき、インデックスは後から有効化できます。`staged` が `true` の場合、インデックスはステージングされた状態になり、staged フラグが削除されるまで有効化されません。ステージングされたインデックスは push の完了をブロックしません。また、ステージングされたインデックスはクエリで使用できません。 |

#### 戻り値 \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, [`Expand`](../modules/server.md#expand)&lt;`SearchIndexes` &amp; `Record`&lt;`IndexName`, &#123; `searchField`: `SearchField` ; `filterFields`: `FilterFields`  &#125;&gt;&gt;, `VectorIndexes`&gt;

この検索インデックスを含む [TableDefinition](server.TableDefinition.md) を返します。

#### 定義場所 \{#defined-in\}

[server/schema.ts:357](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L357)

▸ **searchIndex**&lt;`IndexName`, `SearchField`, `FilterFields`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

このテーブルにステージングされた検索インデックスを定義します。

大きなテーブルでは、インデックスのバックフィルに時間がかかる場合があります。インデックスをステージングしておくと、
先にスキーマを push しておき、インデックスは後から有効化できます。

`staged` が `true` の場合、インデックスはステージングされ、有効化されません。
`staged` フラグが削除されるまで有効にならないということです。ステージングされたインデックスは push
の完了をブロックしません。ステージングされたインデックスはクエリでは使用できません。

検索インデックスについて詳しくは、[Search](https://docs.convex.dev/text-search) を参照してください。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `IndexName` | extends `string` |
| `SearchField` | extends `any` |
| `FilterFields` | extends `any` = `never` |

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `IndexName` | インデックスの名前。 |
| `indexConfig` | `Object` | 検索インデックスの設定オブジェクト。 |
| `indexConfig.searchField` | `SearchField` | 全文検索のためにインデックスするフィールド。このフィールドは `string` 型である必要があります。 |
| `indexConfig.filterFields?` | `FilterFields`[] | 検索クエリを実行する際に、高速なフィルタリングのためにインデックスする追加フィールド。 |
| `indexConfig.staged` | `true` | インデックスをステージ状態にするかどうか。大きなテーブルでは、インデックスのバックフィル処理は時間がかかる場合があります。インデックスをステージすることで、まずスキーマを push し、インデックスは後から有効にできます。`staged` が `true` の場合、インデックスはステージされ、ステージフラグが解除されるまで有効になりません。ステージされたインデックスは push の完了をブロックしません。ステージされたインデックスはクエリで使用できません。 |

#### 戻り値 \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

この検索インデックスを含む [TableDefinition](server.TableDefinition.md)。

#### 定義場所 \{#defined-in\}

[server/schema.ts:401](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L401)

***

### vectorIndex \{#vectorindex\}

▸ **vectorIndex**&lt;`IndexName`, `VectorField`, `FilterFields`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, [`Expand`](../modules/server.md#expand)&lt;`VectorIndexes` &amp; `Record`&lt;`IndexName`, &#123; `vectorField`: `VectorField` ; `dimensions`: `number` ; `filterFields`: `FilterFields`  &#125;&gt;&gt;&gt;

このテーブルに対してベクターインデックスを定義します。

ベクターインデックスについて詳しくは、[Vector Search](https://docs.convex.dev/vector-search) を参照してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `IndexName` | extends `string` |
| `VectorField` | extends `any` |
| `FilterFields` | extends `any` = `never` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `IndexName` | インデックスの名前。 |
| `indexConfig` | `Object` | ベクトルインデックスの設定オブジェクト。 |
| `indexConfig.vectorField` | `VectorField` | ベクトル検索のためにインデックスするフィールド。このフィールドは型 `v.array(v.float64())`（またはそのユニオン型）である必要があります。 |
| `indexConfig.dimensions` | `number` | インデックスされるベクトルの長さ。2 以上 2048 以下である必要があります。 |
| `indexConfig.filterFields?` | `FilterFields`[] | ベクトル検索の実行時に高速にフィルタリングするためにインデックスする追加フィールド。 |
| `indexConfig.staged?` | `false` | インデックスをステージングするかどうか。大きなテーブルでは、インデックスのバックフィルに時間がかかることがあります。インデックスをステージングしておくと、スキーマを push したうえで、インデックスは後から有効化できます。`staged` が `true` の場合、インデックスはステージングされた状態となり、ステージングフラグが削除されるまで有効化されません。ステージングされたインデックスは push の完了をブロックしません。また、ステージングされたインデックスはクエリでは使用できません。 |

#### 戻り値 \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, [`Expand`](../modules/server.md#expand)&lt;`VectorIndexes` &amp; `Record`&lt;`IndexName`, &#123; `vectorField`: `VectorField` ; `dimensions`: `number` ; `filterFields`: `FilterFields`  &#125;&gt;&gt;&gt;

このベクターインデックスを含む [TableDefinition](server.TableDefinition.md)。

#### 定義場所 \{#defined-in\}

[server/schema.ts:448](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L448)

▸ **vectorIndex**&lt;`IndexName`, `VectorField`, `FilterFields`&gt;(`name`, `indexConfig`): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

このテーブルにステージングされたベクターインデックスを定義します。

大きなテーブルでは、インデックスのバックフィルに時間がかかることがあります。インデックスをステージングしておくと、
スキーマを push して、インデックスは後から有効化できます。

`staged` が `true` の場合、インデックスはステージングされた状態になり、有効化されません。
ステージングフラグが削除されるまで有効にはなりません。ステージングされたインデックスは push
の完了をブロックせず、クエリでも使用できません。

ベクターインデックスの詳細については、[Vector Search](https://docs.convex.dev/vector-search) を参照してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `IndexName` | extends `string` |
| `VectorField` | extends `any` |
| `FilterFields` | extends `any` = `never` |

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `name` | `IndexName` | インデックスの名前。 |
| `indexConfig` | `Object` | ベクターインデックスの設定オブジェクト。 |
| `indexConfig.vectorField` | `VectorField` | ベクター検索でインデックスするフィールド。これは `v.array(v.float64())`（またはユニオン）型のフィールドである必要があります。 |
| `indexConfig.dimensions` | `number` | インデックスされるベクターの長さ。2 以上 2048 以下である必要があります。 |
| `indexConfig.filterFields?` | `FilterFields`[] | ベクター検索時の高速なフィルタリングのためにインデックスする追加フィールド。 |
| `indexConfig.staged` | `true` | インデックスをステージ状態にするかどうか。大きなテーブルではインデックスのバックフィルに時間がかかることがあります。インデックスをステージすると、スキーマを push しておき、後からインデックスを有効化できます。`staged` が `true` の場合、インデックスはステージされ、ステージフラグが削除されるまで有効になりません。ステージされたインデックスは push の完了をブロックしません。また、ステージされたインデックスはクエリでは使用できません。 |

#### 戻り値 \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

このベクターインデックスを含む [TableDefinition](server.TableDefinition.md) を返します。

#### 定義場所 \{#defined-in\}

[server/schema.ts:491](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L491)

***

### self \{#self\}

▸ `Protected` **self**(): [`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

https://github.com/microsoft/TypeScript/issues/57035 に対するワークアラウンド。

#### 戻り値 \{#returns\}

[`TableDefinition`](server.TableDefinition.md)&lt;`DocumentType`, `Indexes`, `SearchIndexes`, `VectorIndexes`&gt;

#### 定義元 \{#defined-in\}

[server/schema.ts:534](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L534)