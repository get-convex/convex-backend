---
id: "server.QueryInitializer"
title: "インターフェイス: QueryInitializer<TableInfo>"
custom_edit_url: null
---

[server](../modules/server.md).QueryInitializer

[QueryInitializer](server.QueryInitializer.md) インターフェイスは、Convex データベーステーブル上で [Query](server.Query.md)
を構築するためのエントリーポイントです。

クエリには次の 2 種類があります:

1. フルテーブルスキャン: [fullTableScan](server.QueryInitializer.md#fulltablescan) で作成されるクエリで、
   テーブル内のすべてのドキュメントを挿入順に走査します。
2. インデックス付きクエリ: [withIndex](server.QueryInitializer.md#withindex) で作成されるクエリで、
   インデックス順にインデックス範囲を走査します。

利便性のために、[QueryInitializer](server.QueryInitializer.md) は [Query](server.Query.md) インターフェイスを拡張しており、
暗黙的にフルテーブルスキャンを開始します。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableInfo` | [`GenericTableInfo`](../modules/server.md#generictableinfo) を拡張 |

## 継承階層 \{#hierarchy\}

* [`Query`](server.Query.md)&lt;`TableInfo`&gt;

  ↳ **`QueryInitializer`**

## メソッド \{#methods\}

### [asyncIterator]

▸ **[asyncIterator]**(): `AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### 戻り値 \{#returns\}

`AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### 継承元 \{#inherited-from\}

[Query](server.Query.md).[[asyncIterator]](server.Query.md#[asynciterator])

#### 定義場所 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es2018.asynciterable.d.ts:38

***

### fullTableScan \{#fulltablescan\}

▸ **fullTableScan**(): [`Query`](server.Query.md)&lt;`TableInfo`&gt;

このテーブル内のすべての値を読み取るクエリです。

このクエリのコストはテーブル全体のサイズに比例するため、
常にごく小さいまま（数百〜数千ドキュメント程度）で、かつ更新頻度が低いテーブルに対してのみ使用してください。

#### Returns \{#returns\}

[`Query`](server.Query.md)&lt;`TableInfo`&gt;

* テーブル内のすべてのドキュメントを走査する [Query](server.Query.md)。

#### 定義場所 \{#defined-in\}

[server/query.ts:40](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L40)

***

### withIndex \{#withindex\}

▸ **withIndex**&lt;`IndexName`&gt;(`indexName`, `indexRange?`): [`Query`](server.Query.md)&lt;`TableInfo`&gt;

このテーブルに定義されたインデックスからドキュメントを読み出してクエリを実行します。

このクエリのコストは、インデックス範囲式に一致するドキュメント数に比例します。

結果はインデックスの順序どおりに返されます。

インデックスについて詳しくは、[Indexes](https://docs.convex.dev/using/indexes) を参照してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `IndexName` | extends `string` | `number` | `symbol` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `indexName` | `IndexName` | クエリを実行するインデックスの名前。 |
| `indexRange?` | (`q`: [`IndexRangeBuilder`](server.IndexRangeBuilder.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, [`NamedIndex`](../modules/server.md#namedindex)&lt;`TableInfo`, `IndexName`&gt;, `0`&gt;) =&gt; [`IndexRange`](../classes/server.IndexRange.md) | 指定された [IndexRangeBuilder](server.IndexRangeBuilder.md) で構築される省略可能なインデックス範囲。インデックス範囲は、クエリの実行時に Convex がどのドキュメントを対象とするかを表すものです。インデックス範囲が指定されていない場合、クエリはインデックス内のすべてのドキュメントを対象とします。 |

#### 戻り値 \{#returns\}

[`Query`](server.Query.md)&lt;`TableInfo`&gt;

* インデックス内のドキュメントを取得するクエリ。

#### 定義元 \{#defined-in\}

[server/query.ts:59](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L59)

***

### withSearchIndex \{#withsearchindex\}

▸ **withSearchIndex**&lt;`IndexName`&gt;(`indexName`, `searchFilter`): [`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

検索インデックスに対してフルテキスト検索を実行してクエリを実行します。

検索クエリは常に、インデックスの `searchField` 内のテキストを検索する必要があります。このクエリでは、インデックスで指定された任意の `filterFields` に対する等価フィルターをオプションで追加できます。

ドキュメントは、検索テキストとのマッチ度に基づく関連性の高い順で返されます。

フルテキスト検索について詳しくは、[Indexes](https://docs.convex.dev/text-search) を参照してください。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `IndexName` | extends `string` | `number` | `symbol` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `indexName` | `IndexName` | クエリを実行する検索インデックスの名前。 |
| `searchFilter` | (`q`: [`SearchFilterBuilder`](server.SearchFilterBuilder.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, [`NamedSearchIndex`](../modules/server.md#namedsearchindex)&lt;`TableInfo`, `IndexName`&gt;&gt;) =&gt; [`SearchFilter`](../classes/server.SearchFilter.md) | 渡された [SearchFilterBuilder](server.SearchFilterBuilder.md) を使って構築される検索フィルター式。実行する全文検索の条件と、検索インデックス内で行う等価条件でのフィルタリングの両方を定義する。 |

#### Returns \{#returns\}

[`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

* 一致するドキュメントを検索し、関連度の高い順に返すクエリ。

#### 定義場所 \{#defined-in\}

[server/query.ts:88](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L88)

***

### order \{#order\}

▸ **order**(`order`): [`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

クエリ結果の並び順を定義します。

昇順には `"asc"`、降順には `"desc"` を使用します。指定しない場合、既定では昇順になります。

#### パラメーター \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `order` | `"asc"` | `"desc"` | 結果を返す際の並び順。 |

#### 戻り値 \{#returns\}

[`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

#### 継承元 \{#inherited-from\}

[Query](server.Query.md).[order](server.Query.md#order)

#### 定義元 \{#defined-in\}

[server/query.ts:149](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L149)

***

### filter \{#filter\}

▸ **filter**(`predicate`): [`QueryInitializer`](server.QueryInitializer.md)&lt;`TableInfo`&gt;

クエリの出力をフィルタリングし、`predicate` が true と評価される値のみを返します。

#### パラメータ \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `predicate` | (`q`: [`FilterBuilder`](server.FilterBuilder.md)&lt;`TableInfo`&gt;) =&gt; [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt; | 渡された [FilterBuilder](server.FilterBuilder.md) を使って構築された [Expression](../classes/server.Expression.md) で、どのドキュメントを残すかを指定します。 |

#### 戻り値 \{#returns\}

[`QueryInitializer`](server.QueryInitializer.md)&lt;`TableInfo`&gt;

* 指定されたフィルタ述語を適用した新しい [OrderedQuery](server.OrderedQuery.md)。

#### 継承元 \{#inherited-from\}

[Query](server.Query.md).[filter](server.Query.md#filter)

#### 定義元 \{#defined-in\}

[server/query.ts:165](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L165)

***

### paginate \{#paginate\}

▸ **paginate**(`paginationOpts`): `Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

`n` 個の結果からなるページ分のデータを読み込み、さらに読み込むための [Cursor](../modules/server.md#cursor) を取得します。

注意: これがリアクティブなクエリ関数から呼び出された場合、返される件数は
`paginationOpts.numItems` と一致しない可能性があります。

`paginationOpts.numItems` はあくまで初期値にすぎません。最初の呼び出し以降は、
`paginate` は元のクエリ範囲内のすべてのアイテムを返します。これにより、
すべてのページが互いに隣接して重なりがない状態に保たれます。

#### パラメータ \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `paginationOpts` | [`PaginationOptions`](server.PaginationOptions.md) | 読み込むアイテム数と開始地点を示すカーソルを含む [PaginationOptions](server.PaginationOptions.md) オブジェクト。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

結果の1ページ分と、ページネーションを続行するためのカーソルを含む [PaginationResult](server.PaginationResult.md)。

#### 継承元 \{#inherited-from\}

[Query](server.Query.md).[paginate](server.Query.md#paginate)

#### 定義場所 \{#defined-in\}

[server/query.ts:194](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L194)

***

### collect \{#collect\}

▸ **collect**(): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

クエリを実行し、すべての結果を配列として返します。

注意: 結果が大量になるクエリを処理する場合は、代わりに `Query` を
`AsyncIterable` として利用する方が望ましい場合がよくあります。

#### Returns \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* クエリのすべての結果を格納した配列。

#### 継承元 \{#inherited-from\}

[Query](server.Query.md).[collect](server.Query.md#collect)

#### 定義場所 \{#defined-in\}

[server/query.ts:206](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L206)

***

### take \{#take\}

▸ **take**(`n`): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

クエリを実行し、最初の `n` 件の結果を返します。

#### パラメーター \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `n` | `number` | 取得する要素数。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* クエリの最初の `n` 件の結果を要素とする配列（または、クエリに `n` 件の結果が存在しない場合は、その時点で取得できる件数までの配列）。

#### 継承元 \{#inherited-from\}

[Query](server.Query.md).[take](server.Query.md#take)

#### 定義場所 \{#defined-in\}

[server/query.ts:215](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L215)

***

### first \{#first\}

▸ **first**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

クエリを実行し、結果があれば最初の結果を返します。

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* クエリの最初の値。クエリが結果を返さなかった場合は `null`。

#### 継承元 \{#inherited-from\}

[Query](server.Query.md).[first](server.Query.md#first)

#### 定義場所 \{#defined-in\}

[server/query.ts:222](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L222)

***

### unique \{#unique\}

▸ **unique**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

クエリを実行し、結果が 1 件のみ存在する場合はその単一の結果を返します。

**`Throws`**

クエリが 2 件以上の結果を返した場合はエラーをスローします。

#### Returns \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* クエリから返される単一の結果。存在しない場合は null です。

#### 継承元 \{#inherited-from\}

[Query](server.Query.md).[unique](server.Query.md#unique)

#### 定義元 \{#defined-in\}

[server/query.ts:230](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L230)