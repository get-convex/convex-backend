---
title: "设置"
slug: "deployment-settings"
sidebar_position: 60
description:
  "配置你的 Convex 部署设置，包括 URL、环境变量、身份验证、备份、集成以及部署管理。"
---

[部署设置页面](https://dashboard.convex.dev/deployment/settings)
可让你查看和配置与特定部署（**生产环境**、你个人的**开发环境（dev）**部署，或
**预览**部署）相关的信息和选项。

## URL 和 deploy key \{#url-and-deploy-key\}

[URL 和 deploy key 页面](https://dashboard.convex.dev/deployment/settings)
显示：

* 此部署托管所在的 URL。某些 Convex 集成在配置时可能需要该
  部署 URL。
* 此部署的 HTTP 操作函数应发送到的 URL。
* 部署的 deploy key，用于
  [与 Netlify 和 Vercel 等构建工具集成](/production/hosting/hosting.mdx)
  以及
  [与 Fivetran 和 Airbyte 同步数据](/production/integrations/streaming-import-export.md)。

![Deployment Settings 仪表盘页面](/screenshots/deployment_settings.png)

## 环境变量 \{#environment-variables\}

[环境变量页面](https://dashboard.convex.dev/deployment/settings/environment-variables)
允许你添加、修改、删除和复制此部署的
[环境变量](/production/environment-variables.mdx)。

![部署设置环境变量页面](/screenshots/deployment_settings_env_vars.png)

## 身份验证 \{#authentication\}

[身份验证页面](https://dashboard.convex.dev/deployment/settings/authentication)
展示了在你的 `auth.config.js` 中为用户
[身份验证](/auth.mdx) 实现所配置的值。

## 备份与恢复 \{#backup-restore\}

在
[备份与恢复页面](https://dashboard.convex.dev/deployment/settings/backups)
中，你可以对部署的数据库和文件存储中的数据进行[备份](/database/backup-restore.mdx)。在该页面上，你可以规划定期备份。

![deployment settings export page](/screenshots/backups.png)

## 集成 \{#integrations\}

“集成”页面允许你配置
[日志流式传输](/production/integrations/integrations.mdx)、
[异常上报](/production/integrations/integrations.mdx) 和
[流式导出](/production/integrations/streaming-import-export.md)
集成。

## 暂停部署 \{#pause-deployment\}

在
[pause deployment 页面](https://dashboard.convex.dev/deployment/settings/pause-deployment)
上，你可以通过暂停按钮
[暂停你的部署](/production/pause-deployment.mdx)。

![deployment settings pause deployment 页面](/screenshots/deployment_settings_pause.png)