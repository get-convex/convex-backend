---
title: "配置部署 URL"
slug: "deployment-urls"
sidebar_label: "部署 URL"
hidden: false
sidebar_position: 5
description: "将你的项目配置为使用 Convex 运行"
---

在 [连接到你的后端](/client/react.mdx#connecting-to-a-backend) 时，
正确配置部署 URL 非常重要。

### 创建 Convex 项目 \{#create-a-convex-project\}

首次运行

```sh
npx convex dev
```

在项目目录下，你将创建一个新的 Convex 项目。

你的新项目包含两个部署：*production* 和 *development*。*development* 部署的 URL 会根据你使用的前端框架或打包工具，保存到 `.env.local` 或 `.env` 文件中。

你可以通过访问 Convex
[仪表盘](https://dashboard.convex.dev)中的
[部署设置](/dashboard/deployments/settings.md)
来查看项目中所有部署的 URL。

### 配置客户端 \{#configure-the-client\}

通过传入 Convex 部署的 URL 来构建一个 Convex React 客户端。
在前端应用中通常只需要一个 Convex 客户端。

```jsx title="src/index.js"
import { ConvexProvider, ConvexReactClient } from "convex/react";

const deploymentURL = import.meta.env.VITE_CONVEX_URL;

const convex = new ConvexReactClient(deploymentURL);
```

虽然可以在代码中将该 URL 写死，但使用环境变量来决定客户端应连接到哪个部署会更方便。

根据你所使用的前端框架或打包工具，选择一个在客户端代码中可访问的环境变量名称。

### 选择环境变量名称 \{#choosing-environment-variable-names\}

为了避免在前端代码中无意暴露机密环境变量，许多打包工具要求你在前端代码中引用的环境变量使用特定前缀。

[Vite](https://vitejs.dev/guide/env-and-mode.html) 要求前端代码中使用的环境变量以 `VITE_` 开头，因此 `VITE_CONVEX_URL` 是一个不错的名称。

[Create React App](https://create-react-app.dev/docs/adding-custom-environment-variables/)
要求前端代码中使用的环境变量以 `REACT_APP_` 开头，所以上面的代码使用了 `REACT_APP_CONVEX_URL`。

[Next.js](https://nextjs.org/docs/basic-features/environment-variables#exposing-environment-variables-to-the-browser)
要求它们以 `NEXT_PUBLIC_` 开头，因此 `NEXT_PUBLIC_CONVEX_URL` 是一个不错的名称。

打包工具在访问这些变量的方式上也各不相同：例如
[Vite 使用 `import.meta.env.VARIABLE_NAME`](https://vitejs.dev/guide/env-and-mode.html)，
而很多其他工具（例如 Next.js）使用类似 Node.js 的
[`process.env.VARIABLE_NAME`](https://nextjs.org/docs/basic-features/environment-variables)。

```jsx
import { ConvexProvider, ConvexReactClient } from "convex/react";

const convex = new ConvexReactClient(process.env.NEXT_PUBLIC_CONVEX_URL);
```

[`.env` 文件](https://www.npmjs.com/package/dotenv) 是在开发和生产环境中为环境变量设置不同取值的一种常见方式。`npx convex dev` 会将部署 URL 保存到对应的 `.env` 文件中，并尝试推断你的项目使用的是哪种打包工具。

```shell title=".env.local"
NEXT_PUBLIC_CONVEX_URL=https://guiltless-dog-960.convex.cloud

# 可能传递给前端的其他环境变量示例
NEXT_PUBLIC_SENTRY_DSN=https://123abc@o123.ingest.sentry.io/1234
NEXT_PUBLIC_LAUNCHDARKLY_SDK_CLIENT_SIDE_ID=01234567890abcdef
```

你可以在后端函数中使用在仪表盘中配置的
[环境变量](/production/environment-variables.mdx)。它们不会从 `.env` 文件中获取值。
