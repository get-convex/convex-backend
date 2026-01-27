---
id: "react_auth0"
title: "Módulo: react-auth0"
custom_edit_url: null
---

Componente de inicio de sesión de React para usar con Auth0.

## Funciones \{#functions\}

### ConvexProviderWithAuth0 \{#convexproviderwithauth0\}

▸ **ConvexProviderWithAuth0**(`«destructured»`): `Element`

Un componente contenedor de React que proporciona un [ConvexReactClient](../classes/react.ConvexReactClient.md)
autenticado mediante Auth0.

Debe estar a su vez envuelto por un `Auth0Provider` configurado de `@auth0/auth0-react`.

Consulta [Convex Auth0](https://docs.convex.dev/auth/auth0) para saber cómo configurar
Convex con Auth0.

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `children` | `ReactNode` |
| › `client` | `IConvexReactClient` |

#### Devuelve \{#returns\}

`Element`

#### Definido en \{#defined-in\}

[react-auth0/ConvexProviderWithAuth0.tsx:26](https://github.com/get-convex/convex-js/blob/main/src/react-auth0/ConvexProviderWithAuth0.tsx#L26)