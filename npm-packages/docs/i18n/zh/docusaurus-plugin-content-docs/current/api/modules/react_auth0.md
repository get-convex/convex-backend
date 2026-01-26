---
id: "react_auth0"
title: "模块：react-auth0"
custom_edit_url: null
---

适用于 Auth0 的 React 登录组件。

## 函数 \{#functions\}

### ConvexProviderWithAuth0 \{#convexproviderwithauth0\}

▸ **ConvexProviderWithAuth0**(`«destructured»`): `Element`

一个 React 包装组件，用于提供一个使用 [ConvexReactClient](../classes/react.ConvexReactClient.md)
并通过 Auth0 完成身份验证的客户端。

它必须被一个已配置好的、来自 `@auth0/auth0-react` 的 `Auth0Provider` 包裹。

关于如何将 Convex 与 Auth0 集成，请参见 [Convex Auth0](https://docs.convex.dev/auth/auth0)。

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children` | `ReactNode` |
| › `client` | `IConvexReactClient` |

#### 返回值 \{#returns\}

`Element`

#### 定义于 \{#defined-in\}

[react-auth0/ConvexProviderWithAuth0.tsx:26](https://github.com/get-convex/convex-js/blob/main/src/react-auth0/ConvexProviderWithAuth0.tsx#L26)