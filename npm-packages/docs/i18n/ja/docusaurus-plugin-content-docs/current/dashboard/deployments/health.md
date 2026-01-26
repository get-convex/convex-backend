---
title: "ヘルス"
slug: "health"
sidebar_position: 0
description:
  "失敗率、キャッシュのパフォーマンス、スケジューラーのステータス、最適化に役立つデプロイメントインサイトなど、Convex デプロイメントのヘルスを監視します。"
---

[Health ページ](https://dashboard.convex.dev/deployment/) は、デプロイメントのランディングページです。このページでは、デプロイメントのヘルスに関する重要な情報を確認できます。

## 失敗率 \{#failure-rate\}

![Failure Rate Card](/screenshots/health_failure_rate.png)

失敗率カードには、直近 1 時間の各分ごとのリクエスト失敗率が表示されます。失敗率は、失敗したリクエスト数を総リクエスト数で割った値として計算されます。

## キャッシュヒット率 \{#cache-hit-rate\}

![キャッシュヒット率カード](/screenshots/health_cache_hit_rate.png)

キャッシュヒット率カードは、過去1時間の1分ごとのキャッシュヒットの割合を表示します。キャッシュヒット率は、キャッシュヒットの回数をリクエストの総数で割って算出されます。

キャッシュヒット率はクエリ関数にのみ適用されます。

## スケジューラーのステータス \{#scheduler-status\}

![スケジューラーのステータスカード](/screenshots/scheduler_overdue.png)

スケジューラーのステータスカードには、
[scheduler](/scheduling/scheduled-functions) のステータスが表示されます。スケジューラーがスケジュール済みタスクの数が多すぎて処理が遅れた場合、ステータスは「Overdue」と表示され、遅延時間が分単位で表示されます。

カード右上のボタンをクリックすると、直近 1 時間のスケジューラーのステータスを示すチャートを表示できます。

![スケジューラーのステータスチャート](/screenshots/scheduler_status.png)

## 最終デプロイ \{#last-deployed\}

![Last Deployed Card](/screenshots/health_last_deployed.png)

「最終デプロイ」カードには、あなたの関数が最後にデプロイされた時刻が表示されます。

## インテグレーション \{#integrations\}

<Admonition type="info">
  インテグレーションは Convex Professional でのみ利用できます。
</Admonition>

![Last Deployed Card](/screenshots/health_integrations.png)

インテグレーションカードには、
[Exception Reporting](/production/integrations/exception-reporting) と
[Log Streams](/production/integrations/log-streams) のインテグレーションのステータスが表示され、
インテグレーションを表示および設定するためのクイックリンクが用意されています。

## インサイト \{#insights\}

![Insights Card](/screenshots/insights.png)

Health ページでは、デプロイメントに関するインサイトも表示され、
パフォーマンスと信頼性を向上させるための提案が示されます。

各インサイトには、問題の説明、そのデプロイメントへの影響
（チャートとイベントログによる）、および問題の詳細と解決方法を
学ぶためのリンクが含まれます。

インサイトをクリックすると、その問題の詳細が開き、より大きなチャートと、
そのインサイトをトリガーしたイベントの一覧が表示されます。

![Insight Breakdown](/screenshots/insights_breakdown.png)

利用可能なインサイトには次のようなものがあります:

* 1 回のトランザクションで
  [バイト数を読み取りすぎている](/production/state/limits#transactions)
  関数。
* 1 回のトランザクションで
  [ドキュメントを読み取りすぎている](/production/state/limits#transactions)
  関数。
* [書き込み競合](/error#1) が発生している関数。