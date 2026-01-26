---
title: "项目"
slug: "projects"
sidebar_position: 10
description: "创建和管理 Convex 项目、设置和部署"
---

![Project settings](/screenshots/projects.png)

一个项目对应一个使用 Convex 的代码库，其中包含一个生产部署，以及每位团队成员各自的一个个人部署。

在[登录页](https://dashboard.convex.dev)上点击某个项目会跳转到该项目的详情页面。

## 创建项目 \{#creating-a-project\}

可以在仪表盘中或使用
[CLI](/cli.md#create-a-new-project) 创建项目。要在仪表盘中创建项目，
点击“Create Project”按钮。

## 项目设置 \{#project-settings\}

你可以在 Projects 页面中，点击每个 Project 卡片上的三点 `⋮` 按钮来访问项目级设置。

![Project card menu](/screenshots/project_menu.png)

在 [Project Settings 页面](https://dashboard.convex.dev/project/settings)，你可以：

* 更新项目的名称和 slug。
* 管理项目的 Admin。更多详情请参见
  [Roles and Permissions](/dashboard/teams.md#roles-and-permissions)。
* 查看项目已消耗的[使用量指标](/dashboard/teams.md#usage)。
* 为生产部署添加[自定义域名](/production/hosting/custom.mdx#custom-domains)。
* 为生产和预览部署生成部署密钥。
* 创建和编辑
  [默认环境变量](/production/environment-variables.mdx#project-environment-variable-defaults)。
* 如果你忘记了 `CONVEX_DEPLOYMENT` 配置，可以查看如何重新获得项目访问权限的说明。
* 永久删除该项目。

![Project settings](/screenshots/project_settings.png)

## 删除项目 \{#deleting-projects\}

要删除项目，点击项目卡片上的三点 `⋮` 按钮，然后选择“Delete”。你也可以在 Project Settings
页面删除你的项目。

项目一旦被删除，将无法恢复。与该项目关联的所有部署和数据
都会被永久移除。从仪表盘删除项目时，系统会要求你确认删除操作。在生产部署中有活动的项目
会有额外的确认步骤，以防止意外删除。

![删除项目](/screenshots/project_delete.png)