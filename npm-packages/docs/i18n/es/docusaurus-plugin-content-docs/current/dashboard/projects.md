---
title: "Proyectos"
slug: "projects"
sidebar_position: 10
description: "Crear y administrar proyectos de Convex, configuración y despliegues"
---

![Configuración de proyectos](/screenshots/projects.png)

Un proyecto corresponde a una base de código que usa Convex y contiene un
despliegue de producción y un despliegue personal para cada miembro del equipo.

Al hacer clic en un proyecto en la [página de inicio](https://dashboard.convex.dev)
se te redirigirá a los detalles del proyecto.

## Creación de un proyecto \{#creating-a-project\}

Los proyectos se pueden crear desde el panel de control o mediante la
[CLI](/cli.md#create-a-new-project). Para crear un proyecto desde el panel de control,
haz clic en el botón Create Project.

## Configuración del proyecto \{#project-settings\}

Puedes acceder a la configuración a nivel de proyecto haciendo clic en el botón
de tres puntos `⋮` en cada tarjeta de proyecto en la página de Proyectos.

![Menú de tarjeta de proyecto](/screenshots/project_menu.png)

En la [página de configuración del proyecto](https://dashboard.convex.dev/project/settings),
puedes:

* Actualizar el nombre y el slug de tu proyecto.
* Administrar los administradores del proyecto. Consulta
  [Roles y permisos](/dashboard/teams.md#roles-and-permissions) para más
  detalles.
* Consultar las [métricas de uso](/dashboard/teams.md#usage) que ha consumido tu proyecto.
* Agregar [dominios personalizados](/production/hosting/custom.mdx#custom-domains) para tu
  despliegue de producción
* Generar claves de despliegue para tus despliegues de producción y de vista previa.
* Crear y editar
  [variables de entorno predeterminadas](/production/environment-variables.mdx#project-environment-variable-defaults).
* Ver instrucciones para recuperar el acceso a tu proyecto, en caso de que pierdas tu configuración
  `CONVEX_DEPLOYMENT`.
* Eliminar el proyecto de forma permanente.

![Configuración del proyecto](/screenshots/project_settings.png)

## Eliminación de proyectos \{#deleting-projects\}

Para eliminar un proyecto, haz clic en el botón de tres puntos `⋮` en la tarjeta del proyecto y
selecciona &quot;Delete&quot;. También puedes eliminar tu proyecto desde la página
&quot;Project Settings&quot;.

Una vez que se haya eliminado un proyecto, no se podrá recuperar. Todos los despliegues y datos
asociados con el proyecto se eliminarán de forma permanente. Al eliminar un proyecto
desde el panel de control, se te pedirá que confirmes la eliminación. Los proyectos con
actividad en el despliegue de producción tendrán pasos de confirmación adicionales para
evitar eliminaciones accidentales.

![Eliminar proyecto](/screenshots/project_delete.png)