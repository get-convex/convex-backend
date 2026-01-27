---
id: "react_auth0"
title: "モジュール: react-auth0"
custom_edit_url: null
---

Auth0 で使用する React 用のログインコンポーネントです。

## 関数 \{#functions\}

### ConvexProviderWithAuth0 \{#convexproviderwithauth0\}

▸ **ConvexProviderWithAuth0**(`«destructured»`): `Element`

Auth0 で認証された [ConvexReactClient](../classes/react.ConvexReactClient.md) を提供する React 用のラッパーコンポーネントです。

`@auth0/auth0-react` の設定済みの `Auth0Provider` でラップする必要があります。

Convex と Auth0 のセットアップ方法については、[Convex Auth0](https://docs.convex.dev/auth/auth0) を参照してください。

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children` | `ReactNode` |
| › `client` | `IConvexReactClient` |

#### 戻り値 \{#returns\}

`Element`

#### 定義元 \{#defined-in\}

[react-auth0/ConvexProviderWithAuth0.tsx:26](https://github.com/get-convex/convex-js/blob/main/src/react-auth0/ConvexProviderWithAuth0.tsx#L26)