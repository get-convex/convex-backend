---
title: "函数"
slug: "functions"
sidebar_position: 10
description:
  "使用指标和性能数据运行、测试和监控 Convex 函数"
---

![函数仪表盘视图](/screenshots/functions.png)

[函数页面](https://dashboard.convex.dev/deployment/functions)
展示了所有当前已部署的 Convex 函数。

对于开发环境（dev）部署，这些函数会通过
[`npx convex dev`](/cli.md#run-the-convex-dev-server) 持续更新。生产部署中的函数则通过
[`npx convex deploy`](/cli.md#deploy-convex-functions-to-production) 注册。

## 运行函数 \{#running-functions\}

要在仪表盘中运行 Convex 函数，从页面左侧的列表中选择一个函数，然后点击出现在函数名称旁边的“Run Function”按钮。

如果你当前不在 Functions 页面，也可以通过所有部署页面右下角显示的常驻 *fn* 按钮打开这个界面。打开函数运行器的键盘快捷键是 Ctrl + `（反引号）。

在这个界面中，你可以填写函数的参数并运行它。

当你修改函数参数或数据发生变化时，查询结果会自动更新。

当你点击“Run”按钮后，变更和操作的结果才会显示出来。

请注意，这些结果会显示该函数输出的日志和返回的值。要查看运行函数时具体发生了哪些更改，请参见
[数据页面](/dashboard/deployments/data.md)。

![运行函数](/screenshots/run_function.png)

你也可以选择“Custom test query”选项，而不是使用已部署的函数之一，来
[编写自定义查询函数](/dashboard/deployments/data.md#writing-custom-queries)。

### 查询分页函数 \{#querying-a-paginated-function\}

在仪表盘中查询分页函数时，UI 会要求参数中包含
[`PaginationOptions`](/api/interfaces/server.PaginationOptions) —— 即一个对象，其中包含 `numItems` 字段，以及可选的 `cursor` 字段。该参数的名称应与在查询函数中定义的参数名保持一致。

* `numItems` 应为每页包含的项目数量
* `cursor` 可以留空以开始分页。当你收到结果后，可以将 `cursor` 设置为结果中的 `continueCursor` 字段以继续跳转到下一页。

### 模拟用户身份 \{#assuming-a-user-identity\}

<Admonition type="tip">
  在 Convex 仪表盘中模拟某个用户身份并不会让你获得真实的用户身份信息。
  相反，你可以把这个功能理解为在你的函数中“mock”一个用户身份。
</Admonition>

如果你正在构建一个需要认证的应用，你可能希望在以某个已认证用户身份
的情况下运行一个 Convex 函数。

为此，勾选 “Act as a user” 复选框。

接下来，你可以在出现的输入框中填写用户身份对象。

![以用户身份执行](/screenshots/acting_as_a_user.png)

可用的用户属性包括：

| 属性                | 类型                                      |
| ------------------- | ----------------------------------------- |
| subject*           | string                                    |
| issuer*            | string                                    |
| name                | string                                    |
| givenName           | string                                    |
| familyName          | string                                    |
| nickname            | string                                    |
| preferredUsername   | string                                    |
| profileUrl          | string                                    |
| email               | string                                    |
| emailVerified       | boolean                                   |
| gender              | string                                    |
| birthday            | string                                    |
| timezone            | string                                    |
| language            | string                                    |
| phoneNumber         | string                                    |
| phoneNumberVerified | boolean                                   |
| address             | string                                    |
| updatedAt           | string（RFC 3339 日期格式）               |
| customClaims        | object                                    |

*这些属性是必填的。

## 指标 \{#metrics\}

每个函数都有四个基本图表。有关团队整体使用情况的指标，请参阅[团队设置](/dashboard/teams.md#usage)。

### 调用次数 \{#invocations\}

该图表展示了你的函数每分钟被调用的次数。随着应用使用量的增加，你应该会看到该图表整体呈上升趋势。

### 错误 \{#errors\}

展示在运行你的函数时发生的所有异常的图表。想知道哪里出了问题？请查看下文详述的日志页面。

### 缓存命中率 \{#cache-hit-rate\}

<Admonition type="tip">
  缓存命中率仅适用于查询函数
</Admonition>

一个百分比指标，用于表示该函数有多大比例是复用缓存的值，
而不是重新执行。缓存命中率越高，你的应用运行得越好，响应时间也会越快。

### Execution Time \{#execution-time\}

此函数运行所花费的时间，以毫秒为单位。

该图表上绘制了四条独立的曲线：p50、p90、p95 和 p99。
每条曲线代表在一段时间内请求耗时分布中对应百分位的响应时间。也就是说，只有 1% 的请求运行时间比
p99 曲线所显示的时间更长。通常，关注这些 *尾部延迟（tail latencies）*
是确保你的应用能够快速获取数据服务的一个好方法。

请同时关注执行时间与缓存命中率之间的关系。一般来说，缓存命中所需时间远低于 1 毫秒，因此缓存命中率越高，
你的响应时间就会越好。

点击任意图表可以打开一个更大且更详细的视图，你可以在其中自定义要查看的时间范围。