---
id: "react_clerk"
title: "Módulo: react-clerk"
custom_edit_url: null
---

Componente de inicio de sesión de React para usar con Clerk.

## Funciones \{#functions\}

### ConvexProviderWithClerk \{#convexproviderwithclerk\}

▸ **ConvexProviderWithClerk**(`«destructured»`): `Element`

Un componente de React contenedor que proporciona un [ConvexReactClient](../classes/react.ConvexReactClient.md)
autenticado con Clerk.

Debe estar envuelto por un `ClerkProvider` configurado, de
`@clerk/clerk-react`, `@clerk/clerk-expo`, `@clerk/nextjs` u
otra biblioteca cliente de Clerk basada en React y recibir el hook
`useAuth` correspondiente.

Consulta [Convex Clerk](https://docs.convex.dev/auth/clerk) para ver cómo configurar
Convex con Clerk.

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children` | `ReactNode` |
| › `client` | `IConvexReactClient` |
| › `useAuth` | `UseAuth` |

#### Devuelve \{#returns\}

`Element`

#### Definido en \{#defined-in\}

[react-clerk/ConvexProviderWithClerk.tsx:41](https://github.com/get-convex/convex-js/blob/main/src/react-clerk/ConvexProviderWithClerk.tsx#L41)