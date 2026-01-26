---
title: "Configurar la URL de implementación"
slug: "deployment-urls"
sidebar_label: "URLs de implementación"
hidden: false
sidebar_position: 5
description: "Configurar tu proyecto para que se ejecute con Convex"
---

Cuando [te conectes a tu backend](/client/react.mdx#connecting-to-a-backend),
es importante configurar correctamente la URL de implementación.

### Crear un proyecto de Convex \{#create-a-convex-project\}

La primera vez que ejecutes

```sh
npx convex dev
```

En el directorio de tu proyecto crearás un nuevo proyecto de Convex.

Tu nuevo proyecto incluye dos despliegues: *production* y *development*. La URL
del despliegue de *development* se guardará en el archivo `.env.local` o `.env`,
según el framework de frontend o empaquetador que utilices.

Puedes encontrar las URL de todos los despliegues de un proyecto visitando la
[página de configuración de despliegues](/dashboard/deployments/settings.md) en tu
[panel de control](https://dashboard.convex.dev) de Convex.

### Configura el cliente \{#configure-the-client\}

Crea un cliente de Convex para React pasando la URL del despliegue de Convex.
Por lo general, debería haber un único cliente de Convex en una aplicación de frontend.

```jsx title="src/index.js"
import { ConvexProvider, ConvexReactClient } from "convex/react";

const deploymentURL = import.meta.env.VITE_CONVEX_URL;

const convex = new ConvexReactClient(deploymentURL);
```

Aunque esta URL se puede definir de forma fija en el código, es conveniente usar una variable de entorno
para determinar a qué despliegue debe conectarse el cliente.

Usa un nombre de variable de entorno accesible desde tu código de cliente en función
del framework de frontend o empaquetador que estés utilizando.

### Elegir nombres para las variables de entorno \{#choosing-environment-variable-names\}

Para evitar exponer accidentalmente variables de entorno secretas en el código de frontend,
muchos bundlers requieren que las variables de entorno referenciadas en el código de frontend usen un
prefijo específico.

[Vite](https://vitejs.dev/guide/env-and-mode.html) requiere que las variables de
entorno usadas en el código de frontend comiencen con `VITE_`, por lo que `VITE_CONVEX_URL` es un
buen nombre.

[Create React App](https://create-react-app.dev/docs/adding-custom-environment-variables/)
requiere que las variables de entorno usadas en el código de frontend comiencen con `REACT_APP_`,
por lo que el código anterior usa `REACT_APP_CONVEX_URL`.

[Next.js](https://nextjs.org/docs/basic-features/environment-variables#exposing-environment-variables-to-the-browser)
requiere que comiencen con `NEXT_PUBLIC_`, por lo que `NEXT_PUBLIC_CONVEX_URL` es un
buen nombre.

Los bundlers también proporcionan distintas formas de acceder a estas variables: mientras que
[Vite usa `import.meta.env.VARIABLE_NAME`](https://vitejs.dev/guide/env-and-mode.html),
muchas otras herramientas como Next.js usan el estilo de Node.js
[`process.env.VARIABLE_NAME`](https://nextjs.org/docs/basic-features/environment-variables).

```jsx
import { ConvexProvider, ConvexReactClient } from "convex/react";

const convex = new ConvexReactClient(process.env.NEXT_PUBLIC_CONVEX_URL);
```

Los archivos [`.env`](https://www.npmjs.com/package/dotenv) son una forma común de configurar
distintos valores de variables de entorno en entornos de desarrollo y producción.
`npx convex dev` guardará la URL de implementación en el archivo `.env`
correspondiente, mientras intenta inferir qué bundler utiliza tu proyecto.

```shell title=".env.local"
NEXT_PUBLIC_CONVEX_URL=https://guiltless-dog-960.convex.cloud

# ejemplos de otras variables de entorno que podrían pasarse al frontend
NEXT_PUBLIC_SENTRY_DSN=https://123abc@o123.ingest.sentry.io/1234
NEXT_PUBLIC_LAUNCHDARKLY_SDK_CLIENT_SIDE_ID=01234567890abcdef
```

Tus funciones de backend pueden usar
[variables de entorno](/production/environment-variables.mdx) configuradas en
tu panel de control. No obtienen valores de archivos `.env`.
