---
title: "日志"
slug: "logs"
sidebar_position: 40
description:
  "在仪表盘中查看实时函数日志和部署活动"
---

![日志仪表盘页面](/screenshots/logs.png)

[日志页面](https://dashboard.convex.dev/deployment/logs) 提供一个实时视图，
用于展示你的部署中发生的所有活动。

日志页面会提供最近函数日志的简要历史记录，并在生成新日志时实时显示。
如果你希望保存更长时间的日志历史记录，可以配置
[日志流](/production/integrations/log-streams/log-streams.mdx)。

函数活动包括：

* 函数执行的时间。
* 函数执行的请求 ID。
* 函数执行的结果（成功或失败）。
* 被调用函数的名称。
* 函数的输出，包括函数记录的任何日志行（例如 `console.log`）以及异常。
* 函数执行的持续时间（以毫秒计，不包含网络延迟）。

除了函数活动之外，描述配置变更的
[部署事件](/dashboard/deployments/history.md) 也会出现在这里。

点击一条日志会打开一个视图，显示与所选日志具有相同 Request ID 的所有日志。
这对于调试错误以及理解一次函数执行的上下文非常有用。

![Request ID 日志](/screenshots/request_logs.png)

你可以使用页面顶部的控件，根据文本、函数名称、执行状态和日志级别来过滤日志。

### 筛选日志 \{#filter-logs\}

使用页面顶部的 “Filter logs...” 输入框来过滤日志文本。

你可以使用 “Functions” 下拉列表在结果中包含或排除某些函数。

你还可以结合 “Filter logs” 和
[Convex request id](/functions/error-handling/error-handling.mdx#debugging-errors)
来查找特定错误的日志。比如，如果你在浏览器控制台中看到如下 `Error`：

![Browser Error](/screenshots/console_error_requestid.png)

你可以在 Convex 仪表盘的
[Logs](/dashboard/deployments/logs.md) 页面中，将该 Request ID 粘贴到
“Search logs...” 搜索栏中，以查看该函数的日志。请注意，由于此页面并不是日志的完整历史视图，你可能无法找到较早请求的日志。

大多数错误上报服务和日志接收端（log sink）也应该可以通过 Request ID 进行搜索。

### 日志类型 \{#log-types\}

日志也可以按类型进行筛选。类型包括函数结果（成功或失败）以及严重性级别（info、warn、debug、error）。

所有失败的执行都会包含失败原因，通常是一个 JavaScript 异常。