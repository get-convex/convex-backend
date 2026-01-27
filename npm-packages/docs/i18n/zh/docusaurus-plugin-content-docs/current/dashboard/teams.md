---
title: "团队"
slug: "teams"
sidebar_position: 0
description:
  "在 Convex 中管理团队设置、成员、计费和访问控制"
---

在 Convex 中，你的项目是按团队组织的。团队用于与你的项目共享访问权限给其他人。你可以通过点击 Convex 仪表盘顶部的团队名称在团队之间切换，或创建新团队。这会打开项目选择器，你可以在其中再次点击团队名称来切换团队。

![团队切换器](/screenshots/team_selector.png)

你可以通过点击项目列表页面顶部的 “Team Settings” 按钮来更改团队名称或邀请新成员加入团队。

## 常规 \{#general\}

[常规页面](https://dashboard.convex.dev/team/settings) 允许更改团队名称和 slug。

你也可以在此页面删除团队。只有在先删除该团队的所有项目，并将所有其他团队成员从团队中移除之后，才能删除该团队。删除你的团队会自动取消你的 Convex 订阅。

![团队常规设置页面](/screenshots/teams_general.png)

## 团队成员 \{#team-members\}

使用
[成员设置页面](https://dashboard.convex.dev/team/settings/members)
邀请或移除团队成员。

![团队成员页面](/screenshots/teams_members.png)

### 角色和权限 \{#roles-and-permissions\}

在管理你团队、项目和部署的访问权限时，Convex 提供了两个控制层次。团队级角色决定用户在团队中可以执行的操作，而项目级权限决定用户在特定项目中可以执行的操作。

#### 团队角色 \{#team-roles\}

你的团队成员可以拥有以下角色之一：

* Admin
* Developer

团队的创建者会自动被赋予 Admin 角色。在邀请新的团队成员时，你可以为他们选择一个角色。你也可以在任何时候更改团队成员的角色。

Developer 角色可以：

* 创建新项目和部署。当创建一个新项目时，该项目的创建者会自动被授予该项目的
  [项目管理员](#project-admins) 角色。
* 查看现有项目，并为这些项目创建开发环境和预览环境的部署。Developer 可以从生产环境部署中读取数据，但不能向其写入数据。
* 查看团队的使用情况和账单状态（例如以往和即将到来的发票）

Admin 角色可以执行 Developer 能做的所有操作，并且还可以：

* 邀请新的团队成员
* 将成员从团队中移除
* 更改其他团队成员的角色
* 管理团队的 Convex 订阅和账单详情
* 更改团队名称和 slug
* 团队 Admin 还会被默认授予团队中所有项目的项目管理员访问权限。更多信息参见
  [项目管理员](#project-admins)。

#### 项目管理员 \{#project-admins\}

除了团队角色之外，你还可以通过为团队成员授予“Project Admin（项目管理员）”角色，对单个项目授予管理员权限。

如果你是某个项目的 Project Admin，你可以：

* 更新项目名称和 slug
* 更新项目的默认环境变量
* 删除项目
* 向生产部署写入数据

你可以在成员设置页面中，同时为多个项目分配或移除 Project Admin 角色。要同时为多个成员分配或移除 Project Admin 角色，请改为访问
[Project Settings](/dashboard/projects.md#project-settings) 页面。

## 计费 \{#billing\}

使用 [计费页面](https://dashboard.convex.dev/team/settings/billing)
将你的 Convex 订阅升级到更高级别的套餐，或管理你现有的订阅。

在付费套餐中，你还可以更新计费联系人信息、支付方式，并查看发票。

[详细了解 Convex 定价](https://www.convex.dev/pricing)。

![团队计费页面](/screenshots/teams_billing.png)

### 支出上限 \{#spending-limits\}

当你拥有有效的 Convex 订阅时，可以在
[结算页面](https://dashboard.convex.dev/team/settings/billing)
为团队设置支出上限：

* **警告阈值**只是一个软限制：如果超出该阈值，团队会通过电子邮件收到通知，但不会采取其他操作。
* **禁用阈值**是一个硬限制：如果超出该阈值，团队中的所有项目都会被禁用。这会导致在尝试运行项目中的函数时抛出错误。你可以通过提高或移除该限制来重新启用项目。

支出上限只适用于你的团队项目在套餐包含额度之外消耗的资源。席位费用（为团队中每位开发者支付的金额）不计入该上限。例如，如果你将支出上限设置为 $0/月，你只会被收取席位费用，并且一旦超出套餐自带的资源额度，你的项目就会被禁用。

![设置了部分支出上限的团队结算页面。](/screenshots/teams_billing_spending_limits.png)

## 使用情况 \{#usage\}

在[使用情况页面](https://dashboard.convex.dev/team/settings/usage)中，你可以
查看你的团队消耗的所有资源，并了解这些使用量与你的套餐限制相比的情况。

[进一步了解 Convex 定价](https://www.convex.dev/pricing)。

![团队使用情况页面](/screenshots/teams_usage.png)

所有指标都提供按天分解的明细：

![团队使用情况页面图表](/screenshots/teams_usage_2.png)

## 审计日志 \{#audit-log\}

<Admonition type="info">
  审计日志仅在 Convex Professional 中提供。
</Admonition>

[审计日志页面](https://dashboard.convex.dev/team/settings/audit-log)
显示团队成员在该团队中执行的所有操作。这包括创建和管理项目与部署、邀请和移除团队成员等。

![团队审计日志页面](/screenshots/teams_audit_log.png)

你也可以在[部署历史页面](/dashboard/deployments/history.md)上查看与部署相关事件的历史记录。