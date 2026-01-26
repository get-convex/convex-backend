---
id: "react_clerk"
title: "模块：react-clerk"
custom_edit_url: null
---

与 Clerk 集成使用的 React 登录组件。

## 函数 \{#functions\}

### ConvexProviderWithClerk \{#convexproviderwithclerk\}

▸ **ConvexProviderWithClerk**(`«destructured»`): `Element`

一个 React 包装组件，用于提供一个使用 Clerk 进行认证的
[ConvexReactClient](../classes/react.ConvexReactClient.md)。

它必须被一个已配置好的 `ClerkProvider` 包裹，该 `ClerkProvider` 来自
`@clerk/clerk-react`、`@clerk/clerk-expo`、`@clerk/nextjs` 或
其他基于 React 的 Clerk 客户端库，并传入对应的
`useAuth` hook。

有关如何将 Convex 与 Clerk 集成并完成配置的说明，请参见
[Convex Clerk](https://docs.convex.dev/auth/clerk)。

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children` | `ReactNode` |
| › `client` | `IConvexReactClient` |
| › `useAuth` | `UseAuth` |

#### 返回值 \{#returns\}

`Element`

#### 定义于 \{#defined-in\}

[react-clerk/ConvexProviderWithClerk.tsx:41](https://github.com/get-convex/convex-js/blob/main/src/react-clerk/ConvexProviderWithClerk.tsx#L41)