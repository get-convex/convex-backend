---
id: "server.ValidatedFunction"
title: "接口：ValidatedFunction<Ctx, ArgsValidator, Returns>"
custom_edit_url: null
---

[server](../modules/server.md).ValidatedFunction

**`Deprecated`**

—— 请参阅 `MutationBuilder` 或类似的类型定义，
了解用于定义 Convex 函数的类型。

定义带有参数验证的 Convex 查询、变更或操作函数。

参数验证允许你断言传入该函数的参数是预期的类型。

示例：

```js
import { query } from "./_generated/server";
import { v } from "convex/values";

export const func = query({
  args: {
    arg: v.string()
  },
  handler: ({ db }, { arg }) => {...},
});
```

**出于安全考虑，在生产环境的应用中应为所有公开函数添加参数验证。**

参见 [UnvalidatedFunction](../modules/server.md#unvalidatedfunction)，了解未进行参数验证的函数。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Ctx` | `Ctx` |
| `ArgsValidator` | extends [`PropertyValidators`](../modules/values.md#propertyvalidators) |
| `Returns` | `Returns` |

## 属性 \{#properties\}

### args \{#args\}

• **args**: `ArgsValidator`

此函数的参数验证器。

这是一个对象，将参数名称映射到使用 [v](../modules/values.md#v) 构造的验证器。

```js
import { v } from "convex/values";

const args = {
  stringArg: v.string(),
  optionalNumberArg: v.optional(v.number()),
}
```

#### 定义于 \{#defined-in\}

[server/registration.ts:528](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L528)

***

### handler \{#handler\}

• **handler**: (`ctx`: `Ctx`, `args`: [`ObjectType`](../modules/values.md#objecttype)&lt;`ArgsValidator`&gt;) =&gt; `Returns`

#### 类型声明 \{#type-declaration\}

▸ (`ctx`, `args`): `Returns`

该函数的实现。

这是一个接收相应上下文和参数并返回某个结果的函数。

##### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `ctx` | `Ctx` | 上下文对象。根据函数类型不同，它会是 `QueryCtx`、`MutationCtx` 或 `ActionCtx` 之一。 |
| `args` | [`ObjectType`](../modules/values.md#objecttype)&lt;`ArgsValidator`&gt; | 此函数的参数对象。它与参数校验器所定义的类型保持一致。 |

##### 返回值 \{#returns\}

`Returns`

#### 定义于 \{#defined-in\}

[server/registration.ts:542](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L542)