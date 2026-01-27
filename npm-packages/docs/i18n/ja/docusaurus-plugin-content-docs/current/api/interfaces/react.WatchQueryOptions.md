---
id: "react.WatchQueryOptions"
title: "インターフェース: WatchQueryOptions"
custom_edit_url: null
---

[react](../modules/react.md).WatchQueryOptions

[watchQuery](../classes/react.ConvexReactClient.md#watchquery) のオプション。

## プロパティ \{#properties\}

### journal \{#journal\}

• `Optional` **journal**: [`QueryJournal`](../modules/browser.md#queryjournal)

このクエリ関数の以前の実行時に生成された（オプションの）ジャーナルです。

同じ名前と引数を持つクエリ関数への既存のサブスクリプションがある場合、このジャーナルは何の影響も与えません。

#### 定義元 \{#defined-in\}

[react/client.ts:241](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L241)