---
title: "CLI"
sidebar_position: 110
slug: "cli"
description: "用于管理 Convex 项目和函数的命令行界面"
---

Convex 命令行界面（CLI）是用于管理 Convex 项目和 Convex 函数的工具。

要安装 CLI，运行：

```sh
npm install convex
```

使用以下命令可以查看完整的命令列表：

```sh
npx convex
```

## 配置 \{#configure\}

### 创建一个新项目 \{#create-a-new-project\}

你第一次运行该命令时

```sh
npx convex dev
```

它会提示你登录账号并创建一个新的 Convex 项目。然后它会创建：

1. `convex/` 目录：这是你的查询和变更函数的存放位置。
2. 包含 `CONVEX_DEPLOYMENT` 变量的 `.env.local` 文件：这是你的 Convex 项目的主要配置，它是你的开发环境（dev）部署的名称。

### 重新创建项目配置 \{#recreate-project-configuration\}

运行

```sh
npx convex dev
```

在未设置 `CONVEX_DEPLOYMENT` 的项目目录中，用于配置新的或现有的项目。

### 退出登录 \{#log-out\}

```sh
npx convex logout
```

从你的设备中删除现有的 Convex 凭证，这样之后运行 `npx convex dev` 等命令时就可以使用不同的 Convex 账户。

## 开发 \{#develop\}

### 运行 Convex 开发服务器 \{#run-the-convex-dev-server\}

```sh
npx convex dev
```

监听本地文件系统。当你更改[函数](/functions.mdx)或
[模式](/database/schemas.mdx) 时，新版本会被推送到你的开发环境（dev）
部署中，并且 `convex/_generated` 中的[生成的类型定义](/generated-api/)会被
更新。默认情况下，你的开发环境（dev）部署的日志会显示在
终端中。

你也可以
[在本地运行一个 Convex 部署](/cli/local-deployments-for-dev.mdx)用于
开发。

### 打开 Convex 仪表盘 \{#open-the-dashboard\}

```sh
npx convex dashboard
```

打开 [Convex 仪表盘](./dashboard)。

### 打开文档 \{#open-the-docs\}

```sh
npx convex docs
```

回到文档！

### 运行 Convex 函数 \{#run-convex-functions\}

```sh
npx convex run <functionName> [args]
```

在你的开发部署中运行公共或内部的 Convex 查询、变更或操作。

参数以 JSON 对象的形式指定。

```sh
npx convex run messages:send '{"body": "hello", "author": "me"}'
```

添加 `--watch` 参数以实时更新查询结果。添加 `--push` 参数以在运行函数之前将本地
代码推送到部署。

使用 `--prod` 参数在项目的生产部署中运行函数。

### 实时查看部署日志 \{#tail-deployment-logs\}

你可以选择如何将开发环境（dev）部署中的日志输出到控制台：

```sh
# Show all logs continuously
npx convex dev --tail-logs always

# Pause logs during deploys to see sync issues (default)
npx convex dev

# 开发时不显示日志
npx convex dev --tail-logs disable

# Tail logs without deploying
npx convex logs
```

改为在 `npx convex logs` 中使用 `--prod` 选项，以实时查看生产部署的日志。

### 从文件导入数据 \{#import-data-from-a-file\}

```sh
npx convex import --table <tableName> <path>
npx convex import <path>.zip
```

参见详细说明和使用场景：
[数据导入](/database/import-export/import.mdx)。

### 将数据导出到文件 \{#export-data-to-a-file\}

```sh
npx convex export --path <directoryPath>
npx convex export --path <filePath>.zip
npx convex export --include-file-storage --path <path>
```

详见说明和使用场景：
[数据导出](/database/import-export/export.mdx)。

### 显示表中的数据 \{#display-data-from-tables\}

```sh
npx convex data  # 列出表
npx convex data <table>
```

在命令行中展示
[仪表盘数据页面](/dashboard/deployments/data.md) 的简洁视图。

该命令支持使用 `--limit` 和 `--order` 选项来更改显示的数据。对于更复杂的过滤条件，请使用仪表盘数据页面或编写
[查询](/database/reading-data/reading-data.mdx)。

除了你的自定义数据表之外，`npx convex data <table>` 命令也支持
[系统表](/database/advanced/system-tables.mdx)，例如 `_storage`。

### 读写环境变量 \{#read-and-write-environment-variables\}

```sh
npx convex env list
npx convex env get <name>
npx convex env set <name> <value>
npx convex env remove <name>
```

查看和更新此部署的环境变量，你也可以在仪表盘的
[环境变量设置页面](/dashboard/deployments/settings.md#environment-variables)
中进行管理。

## 部署 \{#deploy\}

### 将 Convex 函数部署到生产环境 \{#deploy-convex-functions-to-production\}

```sh
npx convex deploy
```

要推送到的目标部署按以下方式确定：

1. 如果设置了 `CONVEX_DEPLOY_KEY` 环境变量（在 CI 中很常见），则目标部署为与该 key 关联的部署。
2. 如果设置了 `CONVEX_DEPLOYMENT` 环境变量（本地开发时很常见），则目标部署是由 `CONVEX_DEPLOYMENT` 指定的部署所属项目的生产部署。这样你就可以在连接到开发部署的同时，将代码部署到生产部署。

该命令将会：

1. 如果通过 `--cmd` 指定了命令，则先运行该命令。该命令中可以使用 CONVEX&#95;URL（或类似）的环境变量：
   ```sh
   npx convex deploy --cmd "npm run build"
   ```
   你可以通过 `--cmd-url-env-var-name` 自定义 URL 环境变量名：
   ```sh
   npx convex deploy --cmd 'npm run build' --cmd-url-env-var-name CUSTOM_CONVEX_URL
   ```
2. 对你的 Convex 函数进行类型检查。
3. 在 `convex/_generated` 目录中重新生成[生成的代码](/generated-api/)。
4. 打包你的 Convex 函数及其依赖。
5. 将你的函数、[索引](/database/reading-data/indexes/indexes.md)和[模式](/database/schemas.mdx)推送到生产环境。

该命令执行成功后，新函数会立即生效。

### 将 Convex 函数部署到 [预览部署](/production/hosting/preview-deployments.mdx) \{#deploy-convex-functions-to-a-preview-deployment\}

```sh
npx convex deploy
```

当使用包含
[预览部署密钥](/cli/deploy-key-types.mdx#deploying-to-preview-deployments)
的 `CONVEX_DEPLOY_KEY` 环境变量运行时，此命令将会：

1. 创建一个新的 Convex 部署。`npx convex deploy` 会在 Vercel、Netlify、GitHub 和 GitLab 环境中推断 Git 分支名称，或者你可以使用 `--preview-create` 选项自定义与新创建部署关联的名称。
   ```
   npx convex deploy --preview-create my-branch-name
   ```

2. 如果使用 `--cmd` 指定了命令，则运行该命令。该命令将可以访问 CONVEX&#95;URL（或类似）的环境变量：

   ```sh
   npx convex deploy --cmd "npm run build"
   ```

   你可以使用 `--cmd-url-env-var-name` 自定义 URL 环境变量名：

   ```sh
   npx convex deploy --cmd 'npm run build' --cmd-url-env-var-name CUSTOM_CONVEX_URL
   ```

3. 对 Convex 函数进行类型检查。

4. 在 `convex/_generated` 目录中重新生成[生成的代码](/generated-api/)。

5. 打包 Convex 函数及其依赖。

6. 将函数、[索引](/database/reading-data/indexes/indexes.md)和[模式](/database/schemas.mdx)推送到该部署。

7. 运行通过 `--preview-run` 指定的函数（类似于 `npx convex dev` 的 `--run` 选项）。

   ```sh
   npx convex deploy --preview-run myFunction
   ```

请参阅 [Vercel](/production/hosting/vercel.mdx#preview-deployments) 或
[Netlify](/production/hosting/netlify.mdx#deploy-previews) 托管指南，了解如何同时设置前端和后端的预览环境。

### 更新已生成的代码 \{#update-generated-code\}

```sh
npx convex codegen
```

`convex/_generated` 目录中的[生成代码](/generated-api/)
包含 TypeScript 类型检查所需的类型。这些代码会在你运行 `npx convex dev` 时按需自动生成，并且应该提交到代码仓库（否则你的代码将无法通过类型检查！）。

在极少数情况下，如果需要重新生成这部分代码（例如在 CI 中确保检入的是正确版本的生成代码），可以使用这个命令。

生成代码时可能需要与某个 Convex 部署通信，以便在 Convex JavaScript 运行时中解析和执行配置文件。这不会修改该部署上正在运行的代码。
