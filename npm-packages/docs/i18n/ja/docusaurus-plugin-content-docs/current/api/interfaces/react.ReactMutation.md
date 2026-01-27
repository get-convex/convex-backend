---
id: "react.ReactMutation"
title: "インターフェース: ReactMutation<Mutation>"
custom_edit_url: null
---

[react](../modules/react.md).ReactMutation

サーバー側で Convex のミューテーション関数を実行するためのインターフェース。

## 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`&gt; |

## 呼び出し可能 \{#callable\}

### ReactMutation \{#reactmutation\}

▸ **ReactMutation**(`...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

サーバー上でミューテーションを実行し、その戻り値を表す `Promise` を返します。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Mutation`&gt; | サーバーに渡すミューテーションの引数。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

サーバーサイド関数呼び出しの戻り値です。

#### 定義元 \{#defined-in\}

[react/client.ts:64](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L64)

## メソッド \{#methods\}

### withOptimisticUpdate \{#withoptimisticupdate\}

▸ **withOptimisticUpdate**&lt;`T`&gt;(`optimisticUpdate`): [`ReactMutation`](react.ReactMutation.md)&lt;`Mutation`&gt;

このミューテーションの一部として適用する楽観的アップデートを定義します。

これは、高速でインタラクティブな UI を実現するために、ローカルのクエリ結果に対して一時的に行う更新です。これにより、サーバー上でミューテーションが実行される前にクエリ結果を更新できます。

ミューテーションが呼び出されると、楽観的アップデートが適用されます。

楽観的アップデートは、クライアントから一時的にクエリを削除し、ミューテーションが完了して新しいクエリ結果が同期されるまでの間、ローディング状態を演出するためにも使用できます。

ミューテーションが完全に完了し、クエリが更新されると、この楽観的アップデートは自動的にロールバックされます。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends [`OptimisticUpdate`](../modules/browser.md#optimisticupdate)&lt;[`FunctionArgs`](../modules/server.md#functionargs)&lt;`Mutation`&gt;&gt; |

#### パラメータ \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `optimisticUpdate` | `T` &amp; `ReturnType`&lt;`T`&gt; extends `Promise`&lt;`any`&gt; ? `"Optimistic update handlers must be synchronous"` : {} | 適用する楽観的更新。 |

#### 戻り値 \{#returns\}

[`ReactMutation`](react.ReactMutation.md)&lt;`Mutation`&gt;

更新が設定された新しい `ReactMutation` を返します。

#### 定義元 \{#defined-in\}

[react/client.ts:87](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L87)