---
title: 生成代码
description:
  "针对你的应用 API 自动生成的 JavaScript 和 TypeScript 代码"
---

Convex 使用代码生成功能来创建针对你的应用数据模型和 API 的专用代码。Convex 会生成带有 TypeScript 类型定义（`.d.ts`）的 JavaScript 文件（`.js`）。

使用 Convex 并不要求必须开启代码生成，但使用生成的代码可以在编辑器中获得更好的自动补全体验，如果你使用 TypeScript，还能获得更强的类型安全性。

要生成代码，运行：

```
npx convex dev
```

这会创建一个 `convex/_generated` 目录，其中包含：

* [`api.js` 和 `api.d.ts`](./api.md)
* [`dataModel.d.ts`](./data-model.md)
* [`server.js` 和 `server.d.ts`](./server.md)
