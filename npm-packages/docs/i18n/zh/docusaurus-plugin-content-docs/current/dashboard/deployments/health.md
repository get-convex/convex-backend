---
title: "健康状况"
slug: "health"
sidebar_position: 0
description:
  "监控你的 Convex 部署的健康状况，包括失败率、缓存性能、调度器状态，以及用于优化的部署洞察。"
---

[健康状况页面](https://dashboard.convex.dev/deployment/) 是你的部署的入口页面。
在此页面上，你可以查看一些关于部署健康状况的重要信息。

## 失败率 \{#failure-rate\}

![失败率卡片](/screenshots/health_failure_rate.png)

失败率卡片按分钟显示过去一小时内每分钟失败请求所占的百分比。失败率的计算方式是失败请求数量除以请求总数。

## 缓存命中率 \{#cache-hit-rate\}

![Cache Hit Rate Card](/screenshots/health_cache_hit_rate.png)

缓存命中率卡片会按分钟显示过去一小时内的缓存命中百分比。缓存命中率的计算方式是缓存命中次数除以请求总数。

缓存命中率仅适用于查询函数。

## 调度器状态 \{#scheduler-status\}

![Scheduler 状态卡片](/screenshots/scheduler_overdue.png)

调度器状态卡片会显示
[scheduler](/scheduling/scheduled-functions) 的状态。如果调度器因为计划任务过多而出现滞后，状态会显示为 &quot;Overdue&quot;，并以分钟为单位显示滞后时间。

你可以点击卡片右上角的按钮，查看过去一小时调度器状态的图表。

![Scheduler 状态图表](/screenshots/scheduler_status.png)

## 最近部署 \{#last-deployed\}

![Last Deployed Card](/screenshots/health_last_deployed.png)

“最近部署”卡片显示你的函数最近一次部署的时间。

## 集成 \{#integrations\}

<Admonition type="info">
  集成功能仅在 Convex Professional 中可用。
</Admonition>

![最近部署卡片](/screenshots/health_integrations.png)

“集成”卡片会显示你用于
[异常报告](/production/integrations/exception-reporting) 和
[日志流](/production/integrations/log-streams) 的集成状态，并提供快捷入口，方便你查看和配置这些集成。

## 洞察 \{#insights\}

![Insights Card](/screenshots/insights.png)

Health 页面还会展示与你的部署相关的洞察，并提供如何提升性能和可靠性的建议。

每条洞察都包含问题描述、该问题对你的部署的影响（通过图表和事件日志展示），以及指向更多资料的链接，帮助你了解问题及其解决方法。

点击某条洞察会打开该问题的详细分析视图，其中包括更大的图表以及触发该洞察的事件列表。

![Insight Breakdown](/screenshots/insights_breakdown.png)

当前可用的洞察包括：

* 在单个事务中
  [读取过多字节](/production/state/limits#transactions)
  的函数。
* 在单个事务中
  [读取过多文档](/production/state/limits#transactions)
  的函数。
* 发生[写入冲突](/error#1)的函数。