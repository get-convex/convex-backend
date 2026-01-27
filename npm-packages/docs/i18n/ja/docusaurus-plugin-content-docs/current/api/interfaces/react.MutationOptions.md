---
id: "react.MutationOptions"
title: "インターフェイス: MutationOptions<Args>"
custom_edit_url: null
---

[react](../modules/react.md).MutationOptions

[ミューテーション](../classes/react.ConvexReactClient.md#mutation)に対するオプション。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Args` | extends `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; |

## プロパティ \{#properties\}

### optimisticUpdate \{#optimisticupdate\}

• `Optional` **optimisticUpdate**: [`OptimisticUpdate`](../modules/browser.md#optimisticupdate)&lt;`Args`&gt;

このミューテーションと一緒に適用される楽観的更新（optimistic update）。

楽観的更新は、ミューテーションがペンディングの間、クエリをローカルに更新します。
ミューテーションが完了すると、その更新はロールバックされます。

#### 定義場所 \{#defined-in\}

[react/client.ts:282](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L282)