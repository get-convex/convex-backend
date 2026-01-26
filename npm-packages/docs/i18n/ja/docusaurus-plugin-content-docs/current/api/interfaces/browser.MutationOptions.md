---
id: "browser.MutationOptions"
title: "インターフェース: MutationOptions"
custom_edit_url: null
---

[browser](../modules/browser.md).MutationOptions

[ミューテーション](../classes/browser.BaseConvexClient.md#mutation)のオプション。

## プロパティ \{#properties\}

### optimisticUpdate \{#optimisticupdate\}

• `Optional` **optimisticUpdate**: [`OptimisticUpdate`](../modules/browser.md#optimisticupdate)&lt;`any`&gt;

このミューテーションとあわせて適用する楽観的更新。

楽観的更新は、ミューテーションが保留中の間、ローカルのクエリを更新します。
ミューテーションが完了すると、この更新は元に戻されます。

#### 定義場所 \{#defined-in\}

[browser/sync/client.ts:210](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L210)