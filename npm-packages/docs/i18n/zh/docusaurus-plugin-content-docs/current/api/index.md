---
id: "index"
title: "Convex"
custom_edit_url: null
---

# Convex \{#convex\}

用于 Convex 的 TypeScript 后端 SDK、客户端库和命令行界面（CLI）。

Convex 是一个后端应用平台，提供构建产品所需的一切。

开始上手请访问 [docs.convex.dev](https://docs.convex.dev)!

或者查看 [Convex demos](https://github.com/get-convex/convex-demos)。

欢迎在此仓库中就 Convex 的 TypeScript/JavaScript 客户端、Convex CLI 或 Convex 平台整体相关内容发起讨论和提交问题（issues）。

也欢迎在 [Convex Discord Community](https://convex.dev/community) 中分享功能需求、产品反馈或一般性问题。

# 结构 \{#structure\}

该包包含多个用于在 Convex 上构建应用的入口点：

* [`convex/server`](https://docs.convex.dev/api/modules/server)：用于定义 Convex 后端函数、数据库模式等的 SDK。
* [`convex/react`](https://docs.convex.dev/api/modules/react)：用于将 Convex 集成到 React 应用中的 Hook 和 `ConvexReactClient`。
* [`convex/browser`](https://docs.convex.dev/api/modules/browser)：用于在其他浏览器环境中使用 Convex 的 `ConvexHttpClient`。
* [`convex/values`](https://docs.convex.dev/api/modules/values)：用于处理存储在 Convex 中的值的实用工具。
* [`convex/react-auth0`](https://docs.convex.dev/api/modules/react_auth0)：用于通过 Auth0 进行用户认证的 React 组件。
* [`convex/react-clerk`](https://docs.convex.dev/api/modules/react_clerk)：用于通过 Clerk 进行用户认证的 React 组件。
* [`convex/nextjs`](https://docs.convex.dev/api/modules/nextjs)：用于 SSR 的服务端辅助工具，可被 Next.js 和其他 React 框架使用。

该包还包含 [`convex`](https://docs.convex.dev/using/cli)，这是用于管理 Convex 项目的命令行界面（CLI）。