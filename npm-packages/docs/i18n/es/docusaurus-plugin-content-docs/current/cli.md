---
title: "CLI"
sidebar_position: 110
slug: "cli"
description: "Interfaz de línea de comandos para gestionar proyectos y funciones de Convex"
---

La interfaz de línea de comandos (CLI) de Convex es tu herramienta para gestionar proyectos y funciones de Convex.

Para instalar la CLI, ejecuta:

```sh
npm install convex
```

Puedes consultar la lista completa de comandos con:

```sh
npx convex
```

## Configuración \{#configure\}

### Crea un nuevo proyecto \{#create-a-new-project\}

La primera vez que lo ejecutes

```sh
npx convex dev
```

te pedirá que inicies sesión en este dispositivo y crees un nuevo proyecto de Convex. Luego creará:

1. El directorio `convex/`: Este es el directorio donde se encuentran tus funciones de consulta y mutación.
2. `.env.local` con la variable `CONVEX_DEPLOYMENT`: Esta es la configuración principal para tu proyecto de Convex. Es el nombre de tu despliegue de desarrollo.

### Vuelve a crear la configuración del proyecto \{#recreate-project-configuration\}

Ejecuta

```sh
npx convex dev
```

en un directorio de proyecto en el que `CONVEX_DEPLOYMENT` no esté definido para configurar un
proyecto nuevo o existente.

### Cerrar sesión \{#log-out\}

```sh
npx convex logout
```

Elimina las credenciales de Convex existentes de tu dispositivo para que los comandos posteriores, como `npx convex dev`, puedan usar una cuenta de Convex diferente.

## Desarrollo \{#develop\}

### Ejecuta el servidor de desarrollo de Convex \{#run-the-convex-dev-server\}

```sh
npx convex dev
```

Supervisa el sistema de archivos local. Cuando cambias una [función](/functions.mdx) o
el [esquema](/database/schemas.mdx), las nuevas versiones se envían a tu despliegue
de dev y se actualizan los [tipos generados](/generated-api/) en `convex/_generated`.
De forma predeterminada, los registros de tu despliegue de dev se muestran en la
terminal.

También es posible
[ejecutar un despliegue de Convex localmente](/cli/local-deployments-for-dev.mdx) para
desarrollo.

### Abre el panel de control \{#open-the-dashboard\}

```sh
npx convex dashboard
```

Abre el [panel de control de Convex](./dashboard).

### Abre la documentación \{#open-the-docs\}

```sh
npx convex docs
```

Vuelve a esta documentación.

### Ejecuta funciones de Convex \{#run-convex-functions\}

```sh
npx convex run <functionName> [args]
```

Ejecuta una consulta, mutación o acción de Convex, pública o interna, en tu
despliegue de desarrollo.

Los argumentos se especifican como un objeto JSON.

```sh
npx convex run messages:send '{"body": "hello", "author": "me"}'
```

Añade `--watch` para actualizar en tiempo real los resultados de una consulta. Añade `--push` para subir el código local al despliegue antes de ejecutar la función.

Usa `--prod` para ejecutar funciones en el despliegue de producción de un proyecto.

### Seguir los registros del despliegue \{#tail-deployment-logs\}

Puedes elegir cómo enviar los registros desde tu despliegue dev a la consola:

```sh
# Show all logs continuously
npx convex dev --tail-logs always

# Pausar registros durante los despliegues para ver problemas de sincronización (predeterminado)
npx convex dev

# Don't display logs while developing
npx convex dev --tail-logs disable

# Tail logs without deploying
npx convex logs
```

En su lugar, usa `--prod` con `npx convex logs` para ver en tiempo real los registros del despliegue de producción.

### Importar datos desde un archivo \{#import-data-from-a-file\}

```sh
npx convex import --table <tableName> <path>
npx convex import <path>.zip
```

Consulta la descripción y los casos de uso en:
[importación de datos](/database/import-export/import.mdx).

### Exportar datos a un archivo \{#export-data-to-a-file\}

```sh
npx convex export --path <directoryPath>
npx convex export --path <filePath>.zip
npx convex export --include-file-storage --path <path>
```

Consulta la descripción y los casos de uso en:
[exportación de datos](/database/import-export/export.mdx).

### Mostrar datos de las tablas \{#display-data-from-tables\}

```sh
npx convex data  # lista las tablas
npx convex data <table>
```

Muestra una vista sencilla de la
[página de datos del panel de control](/dashboard/deployments/data.md) en la línea de comandos.

El comando admite las opciones `--limit` y `--order` para modificar los datos que se muestran. Para
filtros más complejos, usa la página de datos del panel de control o escribe una
[consulta](/database/reading-data/reading-data.mdx).

El comando `npx convex data &lt;table&gt;` funciona con
[tablas del sistema](/database/advanced/system-tables.mdx), como `_storage`, además de tus propias tablas.

### Leer y escribir variables de entorno \{#read-and-write-environment-variables\}

```sh
npx convex env list
npx convex env get <name>
npx convex env set <name> <value>
npx convex env remove <name>
```

Consulta y actualiza las variables de entorno del despliegue, que también puedes
gestionar en el panel de control, en la página de
[configuración de variables de entorno](/dashboard/deployments/settings.md#environment-variables).

## Despliegue \{#deploy\}

### Desplegar funciones de Convex en producción \{#deploy-convex-functions-to-production\}

```sh
npx convex deploy
```

El despliegue de destino al que se hará push se determina así:

1. Si la variable de entorno `CONVEX_DEPLOY_KEY` está definida (lo habitual en CI),
   entonces se usa el despliegue asociado con esa clave.
2. Si la variable de entorno `CONVEX_DEPLOYMENT` está definida (lo habitual durante
   el desarrollo local), entonces el despliegue de destino es el despliegue de producción
   del proyecto al que pertenece el despliegue especificado por `CONVEX_DEPLOYMENT`.
   Esto te permite desplegar a tu despliegue de producción mientras desarrollas
   contra tu despliegue de dev.

Este comando hará lo siguiente:

1. Ejecutará un comando si se especifica con `--cmd`. El comando tendrá disponible
   la variable de entorno CONVEX&#95;URL (o similar):
   ```sh
   npx convex deploy --cmd "npm run build"
   ```
   Puedes personalizar el nombre de la variable de entorno de la URL con
   `--cmd-url-env-var-name`:
   ```sh
   npx convex deploy --cmd 'npm run build' --cmd-url-env-var-name CUSTOM_CONVEX_URL
   ```
2. Verificará los tipos de tus funciones de Convex.
3. Regenerará el [código generado](/generated-api/) en el directorio
   `convex/_generated`.
4. Empaquetará tus funciones de Convex y sus dependencias.
5. Enviará tus funciones, [índices](/database/reading-data/indexes/indexes.md)
   y [esquema](/database/schemas.mdx) a producción.

Una vez que este comando se ejecute correctamente, las nuevas funciones estarán disponibles de inmediato.

### Despliega las funciones de Convex en un [despliegue de vista previa](/production/hosting/preview-deployments.mdx) \{#deploy-convex-functions-to-a-preview-deployment\}

```sh
npx convex deploy
```

When run with the `CONVEX_DEPLOY_KEY` environment variable containing a
[Preview Deploy Key](/cli/deploy-key-types.mdx#deploying-to-preview-deployments),
este comando hará lo siguiente:

1. Crear un nuevo despliegue de Convex. `npx convex deploy` inferirá el nombre
   de la rama de Git en entornos de Vercel, Netlify, GitHub y GitLab, o se
   puede usar la opción `--preview-create` para personalizar el nombre
   asociado con el despliegue recién creado.
   ```
   npx convex deploy --preview-create my-branch-name
   ```

2. Ejecutar un comando si se especifica con `--cmd`. El comando tendrá
   disponible la variable de entorno CONVEX&#95;URL (o similar):

   ```sh
   npx convex deploy --cmd "npm run build"
   ```

   Puedes personalizar el nombre de la variable de entorno de la URL con
   `--cmd-url-env-var-name`:

   ```sh
   npx convex deploy --cmd 'npm run build' --cmd-url-env-var-name CUSTOM_CONVEX_URL
   ```

3. Verificar los tipos de tus funciones de Convex.

4. Regenerar el [código generado](/generated-api/) en el directorio
   `convex/_generated`.

5. Empaquetar tus funciones de Convex y sus dependencias.

6. Enviar tus funciones, [índices](/database/reading-data/indexes/indexes.md)
   y [esquema](/database/schemas.mdx) al despliegue.

7. Ejecutar una función especificada por `--preview-run` (similar a la opción
   `--run` para `npx convex dev`).

   ```sh
   npx convex deploy --preview-run myFunction
   ```

Consulta la guía de hosting de [Vercel](/production/hosting/vercel.mdx#preview-deployments) o
[Netlify](/production/hosting/netlify.mdx#deploy-previews) para configurar las
vistas previas de frontend y backend juntas.

### Actualiza el código generado \{#update-generated-code\}

```sh
npx convex codegen
```

El [código generado](/generated-api/) en el directorio `convex/_generated`
incluye los tipos necesarios para la comprobación de tipos de TypeScript. Este código se genera
cada vez que es necesario al ejecutar `npx convex dev` y deberías
añadirlo al repositorio (¡tu código no pasará la comprobación de tipos sin él!).

En los raros casos en que sea útil regenerar el código (por ejemplo, en CI para garantizar que se
ha comprobado el código correcto) puedes usar este comando.

Generar código puede requerir comunicarse con un despliegue de Convex para
evaluar archivos de configuración en el runtime de JavaScript de Convex. Esto no
modifica el código que se está ejecutando en el despliegue.
