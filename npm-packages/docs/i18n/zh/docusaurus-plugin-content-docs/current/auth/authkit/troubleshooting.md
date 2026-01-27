---
title: "AuthKit 故障排除"
sidebar_label: "故障排除"
sidebar_position: 30
description: "调试在 Convex 中使用 AuthKit 进行身份验证时遇到的问题"
---

## 平台未获授权 \{#platform-not-authorized\}

```
WorkOSPlatformNotAuthorized: Your WorkOS platform API key is not authorized to
access this team. Please ensure the API key has the correct permissions in the
WorkOS dashboard.
```

当你的 WorkOS 平台 API 密钥无权访问与你的 Convex 团队关联的 WorkOS 团队时，就会出现此错误。

这通常发生在从 WorkOS 工作区中移除了 Convex 时。

你可以联系 WorkOS 支持，请求恢复该权限，或者取消关联当前工作区并创建一个新的工作区：

```bash
npx convex integration workos disconnect-team
npx convex integration workos provision-team
```

你需要使用另一个电子邮箱地址来创建新的 WorkOS 工作区，因为一个电子邮箱地址只能关联一个 WorkOS 工作区。
