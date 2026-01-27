---
title: "计划任务"
slug: "schedules"
sidebar_position: 30
description:
  "在部署中监控和管理计划函数与 cron 任务"
---

[计划任务页面](https://dashboard.convex.dev/deployment/schedules)会显示你部署中的所有[计划函数](/scheduling/scheduled-functions.mdx)和
[cron 任务](/scheduling/cron-jobs.mdx)。使用此页面顶部的选项卡在计划函数和 cron 任务之间切换。

## 定时函数界面 \{#scheduled-functions-ui\}

定时函数界面会显示所有即将执行的函数调用列表。
在这里，你可以筛选某个特定函数的定时运行，并取消
已计划的函数运行。

![Scheduled functions](/screenshots/scheduled_functions.png)

## Cron 任务界面 \{#cron-jobs-ui\}

Cron 任务界面会列出你所有的 Cron 任务，包括每个任务的运行频率和计划运行时间。

![Cron jobs](/screenshots/cron_jobs.png)

展开某个 Cron 任务可以查看该任务的执行历史。

![Cron job history](/screenshots/cron_job_history.png)