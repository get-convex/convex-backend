---
title: "スケジュール"
slug: "schedules"
sidebar_position: 30
description:
  "デプロイメント内のスケジュールされた関数と cron ジョブを監視および管理します"
---

[Schedules ページ](https://dashboard.convex.dev/deployment/schedules)には、
デプロイメント内のすべての
[スケジュールされた関数](/scheduling/scheduled-functions.mdx)と
[cron ジョブ](/scheduling/cron-jobs.mdx)が表示されます。このページ上部のタブを使用して、
スケジュールされた関数と cron ジョブを切り替えることができます。

## スケジュールされた関数の UI \{#scheduled-functions-ui\}

スケジュールされた関数の UI には、今後実行予定の関数呼び出しが一覧表示されます。
ここから、特定の関数のスケジュール済み実行だけを表示したり、
スケジュールされた実行をキャンセルしたりできます。

![Scheduled functions](/screenshots/scheduled_functions.png)

## Cron ジョブ UI \{#cron-jobs-ui\}

Cron ジョブ UI には、すべての Cron ジョブが、その実行頻度と
スケジュールされた実行時刻とともに一覧表示されます。

![Cron jobs](/screenshots/cron_jobs.png)

特定の Cron ジョブを展開すると、選択したジョブの実行履歴が表示されます。

![Cron job history](/screenshots/cron_job_history.png)