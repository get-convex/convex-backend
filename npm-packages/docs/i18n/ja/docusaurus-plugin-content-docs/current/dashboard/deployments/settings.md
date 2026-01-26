---
title: "設定"
slug: "deployment-settings"
sidebar_position: 60
description:
  "URL、環境変数、認証、バックアップ、連携、デプロイメント管理など、Convex デプロイメントの設定を行います。"
---

[デプロイメント設定ページ](https://dashboard.convex.dev/deployment/settings)では、
特定のデプロイメント（**production**、あなたの個人用の **development** デプロイメント、または
**preview** デプロイメント）に関連する情報や設定項目にアクセスできます。

## URL とデプロイキー \{#url-and-deploy-key\}

[「URL とデプロイキー」ページ](https://dashboard.convex.dev/deployment/settings)では、次の内容を確認できます。

* このデプロイメントがホストされている URL。一部の Convex の連携機能では、設定のために
  デプロイメントURLが必要になります。
* このデプロイメント向けの HTTP アクションを送信する先の URL。
* デプロイメントのデプロイキー。これは
  [Netlify や Vercel などのビルドツールと連携する](/production/hosting/hosting.mdx)
  ため、または
  [Fivetran や Airbyte とのデータ同期](/production/integrations/streaming-import-export.md)
  に使用されます。

![デプロイメント設定ダッシュボードページ](/screenshots/deployment_settings.png)

## 環境変数 \{#environment-variables\}

[環境変数ページ](https://dashboard.convex.dev/deployment/settings/environment-variables)では、デプロイメントの
[環境変数](/production/environment-variables.mdx)を追加・変更・削除・コピーできます。

![deployment settings environment variables page](/screenshots/deployment_settings_env_vars.png)

## 認証 \{#authentication\}

[認証ページ](https://dashboard.convex.dev/deployment/settings/authentication)では、ユーザー[認証](/auth.mdx)の実装に用いる、`auth.config.js` で設定されている値が表示されます。

## バックアップと復元 \{#backup-restore\}

[バックアップと復元のページ](https://dashboard.convex.dev/deployment/settings/backups)では、
デプロイメントのデータベースおよびファイルストレージに保存されているデータを
[バックアップ](/database/backup-restore.mdx)できます。このページでは、定期的な
バックアップをスケジュール設定できます。

![デプロイメント設定のエクスポートページ](/screenshots/backups.png)

## インテグレーション \{#integrations\}

インテグレーションページでは、
[ログストリーミング](/production/integrations/integrations.mdx)、
[例外レポート](/production/integrations/integrations.mdx)、および
[ストリーミングエクスポート](/production/integrations/streaming-import-export.md)
との連携を設定できます。

## デプロイメントを一時停止する \{#pause-deployment\}

[デプロイメント一時停止ページ](https://dashboard.convex.dev/deployment/settings/pause-deployment)
では、一時停止ボタンを使って[デプロイメントを一時停止](/production/pause-deployment.mdx)
できます。

![deployment settings pause deployment page](/screenshots/deployment_settings_pause.png)