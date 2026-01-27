---
id: "browser.OptimisticLocalStore"
title: "インターフェイス: OptimisticLocalStore"
custom_edit_url: null
---

[browser](../modules/browser.md).OptimisticLocalStore

楽観的更新で使用するための、Convex クライアント内に現在あるクエリ結果に対するビューです。

## メソッド \{#methods\}

### getQuery \{#getquery\}

▸ **getQuery**&lt;`Query`&gt;(`query`, `...args`): `undefined` | [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;

クライアントからクエリ結果を取得します。

重要: クエリ結果は変更不可（イミュータブル）なものとして扱う必要があります！
クエリ結果内のデータ構造は常に新しいコピーを作成して使用し、
クライアント内のデータを壊してしまわないようにしてください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | 取得するクエリを指定するための [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; | このクエリ用の引数オブジェクト。 |

#### Returns \{#returns\}

`undefined` | [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;

クエリの結果、またはクエリが現在クライアント側に存在しない場合は `undefined` を返します。

#### 定義元 \{#defined-in\}

[browser/sync/optimistic&#95;updates.ts:28](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/optimistic_updates.ts#L28)

***

### getAllQueries \{#getallqueries\}

▸ **getAllQueries**&lt;`Query`&gt;(`query`): &#123; `args`: [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Query`&gt; ; `value`: `undefined` | [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;  &#125;[]

指定された名前を持つすべてのクエリの結果と引数を取得します。

これは、多くのクエリ結果を確認して更新する必要がある複雑な楽観的更新（たとえばページングされたリストの更新）に役立ちます。

重要: クエリ結果は不変のものとして扱う必要があります！
クライアント側のデータ破損を避けるため、クエリ結果内のデータ構造は必ず新しいコピーを作成してから変更してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | 取得するクエリを指定するための [FunctionReference](../modules/server.md#functionreference)。 |

#### Returns \{#returns\}

&#123; `args`: [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Query`&gt; ; `value`: `undefined` | [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;  &#125;[]

指定された名前を持つ各クエリに対応するオブジェクトからなる配列です。
各オブジェクトには次のプロパティが含まれます:

* `args` - クエリに渡された引数オブジェクト。
  * `value` クエリの結果。クエリが読み込み中の場合は `undefined`。

#### 定義場所 \{#defined-in\}

[browser/sync/optimistic&#95;updates.ts:49](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/optimistic_updates.ts#L49)

***

### setQuery \{#setquery\}

▸ **setQuery**&lt;`Query`&gt;(`query`, `args`, `value`): `void`

クエリの結果を楽観的に更新します。

渡せるのは、新しい値（[getQuery](browser.OptimisticLocalStore.md#getquery) の結果である古い値から導出したものでもよい）か、クエリを削除するための `undefined` のいずれかです。
クエリを削除すると、Convex がクエリ結果を再計算している間のローディング状態を表現するのに便利です。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `query` | `Query` | 設定するクエリを指定する [FunctionReference](../modules/server.md#functionreference)。 |
| `args` | [`FunctionArgs`](../modules/server.md#functionargs)&lt;`Query`&gt; | このクエリ用の引数オブジェクト。 |
| `value` | `undefined` | [`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt; | クエリに新しく設定する値、またはクライアントから削除する場合は `undefined`。 |

#### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[browser/sync/optimistic&#95;updates.ts:69](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/optimistic_updates.ts#L69)