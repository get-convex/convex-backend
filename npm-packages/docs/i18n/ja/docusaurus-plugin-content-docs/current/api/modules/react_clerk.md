---
id: "react_clerk"
title: "モジュール: react-clerk"
custom_edit_url: null
---

Clerk で使用する React 用ログインコンポーネント。

## 関数 \{#functions\}

### ConvexProviderWithClerk \{#convexproviderwithclerk\}

▸ **ConvexProviderWithClerk**(`«destructured»`): `Element`

[ConvexReactClient](../classes/react.ConvexReactClient.md) を
Clerk で認証された状態で提供するためのラッパーとなる React コンポーネントです。

このコンポーネントは、設定済みの `ClerkProvider` によってラップされている必要があります。
`ClerkProvider` は `@clerk/clerk-react`、`@clerk/clerk-expo`、`@clerk/nextjs` などの
React ベースの Clerk クライアントライブラリから提供され、その `useAuth` フックを
このコンポーネントに渡す必要があります。

Convex と Clerk のセットアップ方法については
[Convex Clerk](https://docs.convex.dev/auth/clerk) を参照してください。

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children` | `ReactNode` |
| › `client` | `IConvexReactClient` |
| › `useAuth` | `UseAuth` |

#### 戻り値 \{#returns\}

`Element`

#### 定義場所 \{#defined-in\}

[react-clerk/ConvexProviderWithClerk.tsx:41](https://github.com/get-convex/convex-js/blob/main/src/react-clerk/ConvexProviderWithClerk.tsx#L41)