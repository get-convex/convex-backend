---
id: "browser.SubscribeOptions"
title: "インターフェース: SubscribeOptions"
custom_edit_url: null
---

[browser](../modules/browser.md).SubscribeOptions

[subscribe](../classes/browser.BaseConvexClient.md#subscribe) 用のオプション。

## プロパティ \{#properties\}

### journal \{#journal\}

• `Optional` **journal**: [`QueryJournal`](../modules/browser.md#queryjournal)

このクエリ関数の前回の実行から生成された（オプションの）ジャーナルです。

同じ名前と引数を持つクエリ関数に既存のサブスクリプションがある場合、このジャーナルは影響を与えません。

#### 定義元 \{#defined-in\}

[browser/sync/client.ts:190](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L190)