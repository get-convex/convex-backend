---
title: "Configuración"
slug: "deployment-settings"
sidebar_position: 60
description:
  "Configura los ajustes de tu despliegue de Convex, incluidas las URL,
  variables de entorno, autenticación, copias de seguridad, integraciones y la administración de despliegues."
---

La [página de configuración del despliegue](https://dashboard.convex.dev/deployment/settings)
proporciona acceso a información y opciones de configuración relacionadas con un
despliegue específico (**producción**, tu despliegue personal de **desarrollo** o un
despliegue de **previsualización**).

## URL y clave de despliegue \{#url-and-deploy-key\}

La [página de URL y clave de despliegue](https://dashboard.convex.dev/deployment/settings)
muestra:

* La URL en la que se aloja este despliegue. Algunas integraciones de Convex pueden requerir la
  URL de implementación para su configuración.
* La URL a la que se deben enviar las acciones HTTP para este despliegue.
* La clave de despliegue de este despliegue, utilizada para
  [integrarse con herramientas de compilación como Netlify y Vercel](/production/hosting/hosting.mdx)
  y para
  [sincronizar datos con Fivetran y Airbyte](/production/integrations/streaming-import-export.md).

![Página de configuración del despliegue en el panel de control](/screenshots/deployment_settings.png)

## Variables de entorno \{#environment-variables\}

La
[página de variables de entorno](https://dashboard.convex.dev/deployment/settings/environment-variables)
te permite añadir, cambiar, eliminar y copiar las
[variables de entorno](/production/environment-variables.mdx) del despliegue.

![página de variables de entorno de la configuración del despliegue](/screenshots/deployment_settings_env_vars.png)

## Autenticación \{#authentication\}

La
[página de autenticación](https://dashboard.convex.dev/deployment/settings/authentication)
muestra los valores configurados en tu `auth.config.js` para implementar la
[autenticación](/auth.mdx) de usuarios.

## Copia de seguridad y restauración \{#backup-restore\}

La
[página de copia de seguridad y restauración](https://dashboard.convex.dev/deployment/settings/backups)
te permite [hacer una copia de seguridad](/database/backup-restore.mdx) de los datos almacenados en la base de datos y el almacenamiento de archivos de tu
despliegue. En esta página, puedes programar copias de seguridad periódicas.

![página de exportación de la configuración del despliegue](/screenshots/backups.png)

## Integraciones \{#integrations\}

La página de integraciones te permite configurar integraciones de
[transmisión de registros](/production/integrations/integrations.mdx),
[notificación de excepciones](/production/integrations/integrations.mdx) y
[exportación en streaming](/production/integrations/streaming-import-export.md).

## Pausar el despliegue \{#pause-deployment\}

En la
[página para pausar el despliegue](https://dashboard.convex.dev/deployment/settings/pause-deployment)
puedes [pausar tu despliegue](/production/pause-deployment.mdx) usando el botón de pausa.

![página de configuración del despliegue con la opción de pausar el despliegue](/screenshots/deployment_settings_pause.png)