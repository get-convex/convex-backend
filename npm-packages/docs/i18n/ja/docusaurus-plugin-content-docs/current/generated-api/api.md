---
title: "api.js"
sidebar_position: 2
description:
  "Convex 関数および内部呼び出し向けに生成された API リファレンス"
---

<Admonition type="caution" title="このコードは自動生成されています">
  これらのエクスポートは `convex` パッケージから直接利用することはできません。

  代わりに `npx convex dev` を実行して `convex/_generated/api.js`
  および `convex/_generated/api.d.ts` を生成する必要があります。
</Admonition>

これらの型は、アプリ用に定義した Convex 関数に固有であるため、
コード生成を実行する必要があります。

コード生成を使用していない場合は、
代わりに [`makeFunctionReference`](/api/modules/server#makefunctionreference) を使用してください。

### api \{#api\}

アプリの公開 Convex API を表す `API` 型のオブジェクトです。

`API` 型には、アプリの Convex 関数の引数および戻り値の型に関する情報が含まれます。

api オブジェクトは、クライアント側の React フックや、他の関数を実行・スケジュールする Convex 関数から利用されます。

```javascript title="src/App.jsx"
import { api } from "../convex/_generated/api";
import { useQuery } from "convex/react";

const data = useQuery(api.messages.list);
```

### internal \{#internal\}

アプリの内部 Convex API を表す、`API` 型の別のオブジェクトです。

```js title="convex/upgrade.js"
import { action } from "../_generated/server";
import { internal } from "../_generated/api";

export default action({
  handler: async ({ runMutation }, { planId, ... }) => {
    // 決済プロバイダー(例: Stripe)を呼び出して顧客に課金
    const response = await fetch(...);
    if (response.ok) {
      // Convex DBでプランを"professional"としてマーク
      await runMutation(internal.plans.markPlanAsProfessional, { planId });
    }
  },
});
```
