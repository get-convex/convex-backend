---
title: "api.js"
sidebar_position: 2
description:
  "为你的 Convex 函数和内部调用自动生成的 API 参考"
---

<Admonition type="caution" title="此代码为自动生成">
  这些导出在 `convex` 包中无法直接使用！

  你需要运行 `npx convex dev` 来创建 `convex/_generated/api.js`
  和 `convex/_generated/api.d.ts`。
</Admonition>

这些类型需要通过运行代码生成得到，因为它们依赖于你为应用定义的
Convex 函数。

如果你不使用代码生成，可以改用
[`makeFunctionReference`](/api/modules/server#makefunctionreference)。

### api \{#api\}

一个类型为 `API` 的对象，用来描述你的应用对外公开的 Convex API。

其 `API` 类型包含了你的应用中 Convex 函数的参数和返回类型信息。

`api` 对象会被客户端的 React Hook 以及用于运行或调度其他函数的 Convex 函数使用。

```javascript title="src/App.jsx"
import { api } from "../convex/_generated/api";
import { useQuery } from "convex/react";

const data = useQuery(api.messages.list);
```

### internal \{#internal\}

另一个类型为 `API` 的对象，用于描述你的应用内部的 Convex API。

```js title="convex/upgrade.js"
import { action } from "../_generated/server";
import { internal } from "../_generated/api";

export default action({
  handler: async ({ runMutation }, { planId, ... }) => {
    // 调用支付提供商(例如 Stripe)对客户收费
    const response = await fetch(...);
    if (response.ok) {
      // 在 Convex 数据库中将计划标记为"专业版"
      await runMutation(internal.plans.markPlanAsProfessional, { planId });
    }
  },
});
```
