---
id: "react.Watch"
title: "インターフェース: Watch<T>"
custom_edit_url: null
---

[react](../modules/react.md).Watch

Convex のクエリ関数の出力を監視するウォッチ。

## 型パラメーター \{#type-parameters\}

| 名前 |
| :------ |
| `T` |

## メソッド \{#methods\}

### onUpdate \{#onupdate\}

▸ **onUpdate**(`callback`): () =&gt; `void`

クエリの出力の監視を開始します。

このクエリを購読し、クエリ結果が変化するたびに
コールバックを呼び出します。

**重要: クライアントがすでに同じ引数でこのクエリを購読している場合、
クエリ結果が更新されるまでこのコールバックは呼び出されません。**
現在のローカル結果を取得するには
[localQueryResult](react.Watch.md#localqueryresult) を呼び出してください。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `callback` | () =&gt; `void` | クエリ結果が変更されるたびに呼び出される関数です。 |

#### Returns \{#returns\}

`fn`

* 購読を破棄する関数。

▸ (): `void`

クエリの出力の監視を開始します。

このクエリを購読し、クエリ結果が変化するたびに
コールバックを呼び出します。

**重要: クライアントがすでに同じ引数でこのクエリを購読している場合、
クエリ結果が更新されるまでこのコールバックは呼び出されません。**
現在のローカル結果を取得するには
[localQueryResult](react.Watch.md#localqueryresult) を呼び出してください。

##### 戻り値 \{#returns\}

`void`

* サブスクリプションを解除する関数。

#### 定義場所 \{#defined-in\}

[react/client.ts:170](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L170)

***

### localQueryResult \{#localqueryresult\}

▸ **localQueryResult**(): `undefined` | `T`

現在のクエリ結果を取得します。

これは、すでにそのクエリを購読しており、
サーバーから結果を受信済みであるか、またはクエリの値が
楽観的に設定されている場合にのみ結果を返します。

**`Throws`**

クエリがサーバー側でエラーになった場合にエラーをスローします。

#### 戻り値 \{#returns\}

`undefined` | `T`

クエリの結果。結果がまだ分からない場合は `undefined` になります。

#### 定義場所 \{#defined-in\}

[react/client.ts:182](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L182)

***

### journal \{#journal\}

▸ **journal**(): `undefined` | [`QueryJournal`](../modules/browser.md#queryjournal)

このクエリの現在の [QueryJournal](../modules/browser.md#queryjournal) を取得します。

このクエリの結果をまだ受け取っていない場合は、`undefined` が返されます。

#### 戻り値 \{#returns\}

`undefined` | [`QueryJournal`](../modules/browser.md#queryjournal)

#### 定義元 \{#defined-in\}

[react/client.ts:194](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L194)