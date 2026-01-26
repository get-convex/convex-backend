---
id: "index"
title: "convex"
custom_edit_url: null
---

# Convex \{#convex\}

SDK de backend de TypeScript, bibliotecas cliente y CLI para Convex.

Convex es la plataforma de backend para aplicaciones con todo lo que necesitas para construir
tu producto.

Empieza en [docs.convex.dev](https://docs.convex.dev)!

O ve las [demos de Convex](https://github.com/get-convex/convex-demos).

Abre debates y issues en este repositorio sobre los clientes TypeScript/JavaScript
de Convex, la CLI de Convex o la plataforma Convex en
general.

También puedes compartir solicitudes de nuevas funcionalidades, comentarios sobre el producto o preguntas generales
en la [comunidad de Convex en Discord](https://convex.dev/community).

# Estructura \{#structure\}

Este paquete incluye varios puntos de entrada para crear aplicaciones con Convex:

* [`convex/server`](https://docs.convex.dev/api/modules/server): SDK para
  definir funciones de backend de Convex, definir un esquema de base de datos, etc.
* [`convex/react`](https://docs.convex.dev/api/modules/react): hooks y un
  `ConvexReactClient` para integrar Convex en aplicaciones React.
* [`convex/browser`](https://docs.convex.dev/api/modules/browser): Un
  `ConvexHttpClient` para usar Convex en otros entornos de navegador.
* [`convex/values`](https://docs.convex.dev/api/modules/values): Utilidades para
  trabajar con valores almacenados en Convex.
* [`convex/react-auth0`](https://docs.convex.dev/api/modules/react_auth0): Un
  componente de React para autenticar usuarios con Auth0.
* [`convex/react-clerk`](https://docs.convex.dev/api/modules/react_clerk): Un
  componente de React para autenticar usuarios con Clerk.
* [`convex/nextjs`](https://docs.convex.dev/api/modules/nextjs): Utilidades del lado del servidor
  para SSR, usables por Next.js y otros frameworks de React.

Este paquete también incluye [`convex`](https://docs.convex.dev/using/cli), la
CLI (interfaz de línea de comandos) para administrar proyectos de Convex.