---
id: "react.ReactAction"
title: "接口：ReactAction<Action>"
custom_edit_url: null
---

[react](../modules/react.md).ReactAction

用于在服务端执行 Convex 操作的接口。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Action` | 继承自 [`FunctionReference`](../modules/server.md#functionreference)&lt;`"action"`&gt; |

## 可调用 \{#callable\}

### ReactAction \{#reactaction\}

▸ **ReactAction**(`...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

在服务器上执行该函数，并返回一个其返回值的 `Promise`。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Action`&gt; | 要传递到服务器的函数参数。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

服务端函数调用的返回值。

#### 定义于 \{#defined-in\}

[react/client.ts:136](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L136)