---
id: "server.OrderedQuery"
title: "インターフェース: OrderedQuery<TableInfo>"
custom_edit_url: null
---

[server](../modules/server.md).OrderedQuery

既に並び順が定義されている [Query](server.Query.md)。

## 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TableInfo` | extends [`GenericTableInfo`](../modules/server.md#generictableinfo) |

## 継承階層 \{#hierarchy\}

* `AsyncIterable`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

  ↳ **`OrderedQuery`**

  ↳↳ [`Query`](server.Query.md)

## メソッド \{#methods\}

### [asyncIterator]

▸ **[asyncIterator]**(): `AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### 戻り値 \{#returns\}

`AsyncIterator`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;, `any`, `undefined`&gt;

#### 継承元 \{#inherited-from\}

AsyncIterable.[asyncIterator]

#### 定義場所 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es2018.asynciterable.d.ts:38

***

### filter \{#filter\}

▸ **filter**(`predicate`): [`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

クエリの出力をフィルター処理し、`predicate` が true と評価される値のみを返します。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `predicate` | (`q`: [`FilterBuilder`](server.FilterBuilder.md)&lt;`TableInfo`&gt;) =&gt; [`ExpressionOrValue`](../modules/server.md#expressionorvalue)&lt;`boolean`&gt; | 指定された [FilterBuilder](server.FilterBuilder.md) を使って構築され、どのドキュメントを保持するかを指定する [Expression](../classes/server.Expression.md)。 |

#### 戻り値 \{#returns\}

[`OrderedQuery`](server.OrderedQuery.md)&lt;`TableInfo`&gt;

* 指定されたフィルタ述語が適用された新しい [OrderedQuery](server.OrderedQuery.md) を返します。

#### 定義元 \{#defined-in\}

[server/query.ts:165](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L165)

***

### paginate \{#paginate\}

▸ **paginate**(`paginationOpts`): `Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

`n` 個の結果からなる 1 ページ分を読み込み、さらに読み込むための [Cursor](../modules/server.md#cursor) を取得します。

注意: これがリアクティブなクエリ関数から呼び出される場合、結果の件数が
`paginationOpts.numItems` と一致しない場合があります。

`paginationOpts.numItems` は初期値に過ぎません。最初の呼び出し以降は、
`paginate` は元のクエリ範囲に含まれるすべてのアイテムを返します。これにより、
すべてのページが互いに隣接し、重複しない状態が保たれます。

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `paginationOpts` | [`PaginationOptions`](server.PaginationOptions.md) | 読み込む項目数と、読み込みを開始するカーソルを含む [PaginationOptions](server.PaginationOptions.md) オブジェクト |

#### 戻り値 \{#returns\}

`Promise`&lt;[`PaginationResult`](server.PaginationResult.md)&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;&gt;

結果の 1 ページ分と、ページネーションを続行するためのカーソルを含む [PaginationResult](server.PaginationResult.md)。

#### 定義場所 \{#defined-in\}

[server/query.ts:194](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L194)

***

### collect \{#collect\}

▸ **collect**(): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

クエリを実行し、結果をすべて配列として返します。

注意：結果件数が多いクエリを処理する場合は、代わりに `Query` を
`AsyncIterable` として使用する方が望ましい場合がよくあります。

#### Returns \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* すべてのクエリ結果を含む配列。

#### 定義元 \{#defined-in\}

[server/query.ts:206](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L206)

***

### take \{#take\}

▸ **take**(`n`): `Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

クエリを実行し、最初の `n` 件の結果を返します。

#### パラメータ \{#parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `n` | `number` | 取得する要素数。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;[]&gt;

* クエリの最初の `n` 個の結果を要素とする配列（クエリの結果が `n` 個未満の場合は、その件数分のみ）。

#### 定義場所 \{#defined-in\}

[server/query.ts:215](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L215)

***

### first \{#first\}

▸ **first**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

クエリを実行し、結果が存在する場合は先頭の結果を返します。

#### Returns \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* クエリ結果の最初の値。クエリが結果を返さなかった場合は `null`。

#### 定義場所 \{#defined-in\}

[server/query.ts:222](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L222)

***

### unique \{#unique\}

▸ **unique**(): `Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

クエリを実行し、結果が 1 件だけ存在する場合はその結果を返します。

**`Throws`**

クエリが複数の結果を返した場合はエラーをスローします。

#### Returns \{#returns\}

`Promise`&lt;`null` | [`DocumentByInfo`](../modules/server.md#documentbyinfo)&lt;`TableInfo`&gt;&gt;

* クエリから返される単一の結果。存在しない場合は `null` を返します。

#### 定義元 \{#defined-in\}

[server/query.ts:230](https://github.com/get-convex/convex-js/blob/main/src/server/query.ts#L230)