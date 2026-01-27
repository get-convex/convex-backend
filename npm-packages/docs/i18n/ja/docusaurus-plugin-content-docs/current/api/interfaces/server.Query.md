---
id: "server.Query"
title: "インターフェース: Query<TableInfo>"
custom_edit_url: null
---

[server](../modules/server.md).Query

[Query](server.Query.md) インターフェースを使用すると、関数はデータベースから値を読み取ることができます。

**ID でオブジェクトを読み込むだけでよい場合は、代わりに `db.get(id)` を使用してください。**

クエリの実行は、次の呼び出しから構成されます。

1. （任意）順序を定義するための [order](server.Query.md#order)
2. （任意）結果を絞り込むための [filter](server.OrderedQuery.md#filter)
3. 結果を取得するための *consumer* メソッド

クエリは遅延評価されます。反復処理が始まるまで何も実行されないため、クエリの構築や拡張はコストがかかりません。結果が反復処理されるにつれてクエリが段階的に実行されるため、途中で反復を打ち切ればクエリのコストも削減されます。

JavaScript を実行してフィルタリングするよりも、`filter` 式を使用する方が効率的です。

|                                              | |
|----------------------------------------------|-|
| **Ordering**                                 | |
| [`order("asc")`](#order)                     | クエリ結果の順序を定義します。 |
|                                              | |
| **Filtering**                                | |
| [`filter(...)`](#filter)                     | ある条件に一致する値だけが含まれるようにクエリ結果をフィルタリングします。 |
|                                              | |
| **Consuming**                                | クエリを実行し、さまざまな方法で結果を返します。 |
| [`[Symbol.asyncIterator]()`](#asynciterator) | クエリの結果は `for await..of` ループを使って反復処理できます。 |
| [`collect()`](#collect)                      | すべての結果を配列として返します。 |
| [`take(n: number)`](#take)                   | 先頭の `n` 件の結果を配列として返します。 |
| [`first()`](#first)                          | 最初の結果を返します。 |
| [`unique()`](#unique)                        | 唯一の結果を返し、結果が複数ある場合は例外をスローします。 |

クエリの書き方について詳しくは、[Querying the Database](https://docs.convex.dev/using/database-queries) を参照してください。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableInfo` | extends [`GenericTableInfo`](../modules/server.md#generictableinfo) |

## 継承階層 \{#hierarchy\}

* [`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

  ↳ **`Query`**

  ↳↳ [`QueryInitializer`](server.QueryInitializer.md)

## メソッド \{#methods\}

### [asyncIterator]

▸ **[asyncIterator]**(): `AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### 戻り値 \{#returns\}

`AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### 継承元 \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[[asyncIterator]](server.OrderedQuery.md#[asynciterator])

#### 定義場所 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es2018.asynciterable.d.ts:38

***

### order \{#order\}

▸ **order**(`order`): [`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

クエリ結果の並び順を指定します。

昇順には `"asc"`、降順には `"desc"` を使用します。指定しない場合、デフォルトは昇順です。

#### パラメーター \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `order` | `"asc"` | `"desc"` | 結果を返す際の並び順。 |

#### 返り値 \{#returns\}

[`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

#### 定義場所 \{#defined-in\}

[server/query.ts:149](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L149)

***

### filter \{#filter\}

▸ **filter**(`predicate`): [`Query`](server.Query.md)&lt;`TableInfo`&gt;

クエリの出力をフィルタリングし、`predicate` が true に評価される値のみを返します。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `predicate` | (`q`: [`FilterBuilder`](server.FilterBuilder.md)&lt;`TableInfo`&gt;) =&gt; [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt; | 指定された [FilterBuilder](server.FilterBuilder.md) を使用して構築され、どのドキュメントを保持するかを指定するための [Expression](../classes/server.Expression.md)。 |

#### 戻り値 \{#returns\}

[`Query`](server.Query.md)&lt;`TableInfo`&gt;

* 指定したフィルタ述語が適用された新しい [OrderedQuery](server.OrderedQuery.md)。

#### 継承元 \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[filter](server.OrderedQuery.md#filter)

#### 定義元 \{#defined-in\}

[server/query.ts:165](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L165)

***

### paginate \{#paginate\}

▸ **paginate**(`paginationOpts`): `Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

`n` 件の結果からなる 1 ページ分を読み込み、さらに続きの結果を読み込むための [Cursor](../modules/server.md#cursor) を取得します。

注意: これがリアクティブなクエリ関数から呼び出された場合、
結果の件数は `paginationOpts.numItems` と一致しない可能性があります。

`paginationOpts.numItems` は初期値に過ぎません。最初の呼び出し以降は、
`paginate` は元のクエリ範囲内のすべてのアイテムを返します。これにより、
すべてのページが互いに隣接し、重なり合わない状態に保たれます。

#### パラメーター \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `paginationOpts` | [`PaginationOptions`](server.PaginationOptions.md) | 読み込むアイテム数と開始位置となるカーソルを指定する [PaginationOptions](server.PaginationOptions.md) オブジェクト。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

結果ページと、ページネーションを続行するためのカーソルを含む [PaginationResult](server.PaginationResult.md)。

#### 継承元 \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[paginate](server.OrderedQuery.md#paginate)

#### 定義場所 \{#defined-in\}

[server/query.ts:194](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L194)

***

### collect \{#collect\}

▸ **collect**(): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

クエリを実行し、すべての結果を配列として返します。

注意: 結果件数が多いクエリを処理する場合は、代わりに `Query` を
`AsyncIterable` として扱う方がよい場合がよくあります。

#### Returns \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* クエリ結果のすべてを含む配列。

#### 継承元 \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[collect](server.OrderedQuery.md#collect)

#### 定義場所 \{#defined-in\}

[server/query.ts:206](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L206)

***

### take \{#take\}

▸ **take**(`n`): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

クエリを実行し、最初の `n` 件の結果を返します。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `n` | `number` | 取得する要素数。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* クエリ結果の先頭から最大 `n` 件の要素を含む配列（クエリ結果が `n` 件未満の場合は、その件数分だけを含みます）。

#### 継承元 \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[take](server.OrderedQuery.md#take)

#### 定義場所 \{#defined-in\}

[server/query.ts:215](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L215)

***

### first \{#first\}

▸ **first**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

クエリを実行し、結果が存在する場合は先頭の結果を返します。

#### 戻り値 \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* クエリ結果の先頭の値。クエリが結果を返さなかった場合は `null`。

#### 継承元 \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[first](server.OrderedQuery.md#first)

#### 定義場所 \{#defined-in\}

[server/query.ts:222](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L222)

***

### unique \{#unique\}

▸ **unique**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

クエリを実行し、結果が 1 件だけ存在する場合はその値を返します。

**`Throws`**

クエリが 1 件を超える結果を返した場合はエラーをスローします。

#### Returns \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* クエリから返される単一の結果。存在しない場合は null を返します。

#### 継承元 \{#inherited-from\}

[OrderedQuery](server.OrderedQuery.md).[unique](server.OrderedQuery.md#unique)

#### 定義場所 \{#defined-in\}

[server/query.ts:230](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L230)