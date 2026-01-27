---
title: "AuthKit のトラブルシューティング"
sidebar_label: "トラブルシューティング"
sidebar_position: 30
description: "Convex 上の AuthKit 認証の問題をデバッグする"
---

## プラットフォームが認可されていません \{#platform-not-authorized\}

```
WorkOSPlatformNotAuthorized: Your WorkOS platform API key is not authorized to
access this team. Please ensure the API key has the correct permissions in the
WorkOS dashboard.
```

このエラーは、使用している WorkOS プラットフォーム API キーが、あなたの Convex チームに紐づいている WorkOS チームへのアクセスを許可されていない場合に発生します。

これは通常、WorkOS ワークスペースから Convex が削除されたときに発生します。

この権限の復元を WorkOS サポートに依頼するか、現在のワークスペースのリンクを解除して新しいワークスペースを作成してください。

```bash
npx convex integration workos disconnect-team
npx convex integration workos provision-team
```

新しい WorkOS ワークスペースを作成するには、別のメールアドレスを使用する必要があります。
1 つのメールアドレスは、1 つの WorkOS ワークスペースにしか関連付けられないためです。
