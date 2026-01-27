---
title: "Equipos"
slug: "teams"
sidebar_position: 0
description:
  "Administrar la configuración del equipo, los miembros, la facturación y el control de acceso en Convex"
---

En Convex, tus proyectos se organizan por equipos. Los equipos se utilizan para compartir el acceso
a tus proyectos con otras personas. Puedes cambiar entre equipos o crear un nuevo
equipo haciendo clic en el nombre de tu equipo ubicado en la parte superior del panel de control de Convex.
Esto abrirá el selector de proyectos, donde puedes cambiar de equipo haciendo clic
nuevamente en el nombre del equipo.

![Team switcher](/screenshots/team_selector.png)

Puedes cambiar el nombre de un equipo o invitar a nuevos miembros a un equipo haciendo clic en
el botón &quot;Team Settings&quot; ubicado en la parte superior de la página de la lista de proyectos.

## General \{#general\}

La [página general](https://dashboard.convex.dev/team/settings) permite cambiar
el nombre y el *slug* del equipo.

También puedes eliminar el equipo desde esta página. Solo podrás eliminar un equipo después
de haber eliminado todos sus proyectos y de haber quitado a los demás miembros del equipo.
Eliminar tu equipo cancelará automáticamente tu suscripción a Convex.

![Página de configuración general del equipo](/screenshots/teams_general.png)

## Miembros del equipo \{#team-members\}

Utiliza la
[página de configuración de miembros](https://dashboard.convex.dev/team/settings/members) para
invitar o eliminar miembros de tu equipo.

![Página de miembros del equipo](/screenshots/teams_members.png)

### Roles y permisos \{#roles-and-permissions\}

Convex tiene dos niveles de control para gestionar el acceso a tu equipo, proyectos y
despliegues. Los roles a nivel de equipo controlan lo que un usuario puede hacer dentro del equipo, mientras que
los permisos a nivel de proyecto controlan lo que un usuario puede hacer dentro de un proyecto concreto.

#### Roles del equipo \{#team-roles\}

Los miembros de tu equipo pueden tener uno de los siguientes roles:

* Administrador
* Desarrollador

Al creador del equipo se le asigna automáticamente el rol de Administrador. Al invitar
a nuevos miembros del equipo, puedes asignarles un rol. También puedes cambiar el rol
de cualquier miembro del equipo en cualquier momento.

Los desarrolladores pueden:

* Crear nuevos proyectos y despliegues. Cuando se crea un nuevo proyecto, al
  creador del proyecto se le concede automáticamente el rol de
  [Administrador del proyecto](#project-admins) para ese proyecto.
* Ver los proyectos existentes y crear despliegues de desarrollo y de vista previa para
  estos proyectos. Los desarrolladores pueden leer datos de despliegues de producción, pero
  no pueden escribir en ellos.
* Ver el uso del equipo y el estado de facturación del equipo (como facturas anteriores y futuras)

Los administradores pueden hacer todo lo que pueden hacer los desarrolladores, además de:

* Invitar a nuevos miembros del equipo
* Eliminar miembros del equipo
* Cambiar el rol de otros miembros del equipo
* Gestionar la suscripción a Convex del equipo y los detalles de facturación.
* Cambiar el nombre y el slug del equipo
* Los administradores del equipo también obtienen implícitamente acceso como administradores de proyecto a todos los proyectos
  dentro del equipo. Consulta [Administradores del proyecto](#project-admins) para obtener más información.

#### Administradores de proyecto \{#project-admins\}

Además de los roles de equipo, también puedes conceder acceso de administrador a
proyectos individuales asignando a los miembros del equipo el rol de &quot;Administrador de proyecto&quot;.

Si eres Administrador de proyecto para un proyecto determinado, puedes:

* Actualizar el nombre y el slug del proyecto
* Actualizar las variables de entorno predeterminadas del proyecto
* Eliminar el proyecto
* Realizar escrituras en despliegues de producción

Puedes asignar y quitar el rol de Administrador de proyecto para varios proyectos al
mismo tiempo en la página de configuración de miembros. Para asignar o quitar el rol de
Administrador de proyecto para varios miembros al mismo tiempo, visita en cambio la página de
[Configuración del proyecto](/dashboard/projects.md#project-settings).

## Facturación \{#billing\}

Usa la [página de facturación](https://dashboard.convex.dev/team/settings/billing) para
cambiar tu suscripción de Convex a un plan superior o administrar tu
suscripción existente.

En los planes de pago, también puedes actualizar la información de contacto de facturación, el método de pago
y ver tus facturas.

[Más información sobre los precios de Convex](https://www.convex.dev/pricing).

![Página de facturación del equipo](/screenshots/teams_billing.png)

### Límites de gasto \{#spending-limits\}

Cuando tengas una suscripción activa a Convex, puedes establecer los límites de
gasto para tu equipo en la
[página de facturación](https://dashboard.convex.dev/team/settings/billing):

* El **umbral de advertencia** es solo un límite flexible: si se supera, se
  notificará al equipo por correo electrónico, pero no se tomará ninguna otra
  medida.
* El **umbral de desactivación** es un límite estricto: si se supera, se
  desactivarán todos los proyectos del equipo. Esto provocará errores al intentar
  ejecutar funciones en tus proyectos. Puedes volver a activar los proyectos
  aumentando o eliminando el límite.

Los límites de gasto solo se aplican a los recursos usados por los proyectos de
tu equipo más allá de las cantidades incluidas en tu plan. Las tarifas por asiento
(el importe que pagas por cada desarrollador de tu equipo) no se contabilizan en los
límites. Por ejemplo, si estableces el límite de gasto en 0 USD/mes, solo se te
cobrarán las tarifas por asiento y los proyectos se desactivarán si superas los
recursos incluidos de base en tu plan.

![La página de facturación del equipo con algunos límites de gasto establecidos.](/screenshots/teams_billing_spending_limits.png)

## Uso \{#usage\}

En la [página de uso](https://dashboard.convex.dev/team/settings/usage) puedes
ver todos los recursos consumidos por tu equipo y cómo vas en relación con
los límites de tu plan.

[Más información sobre los precios de Convex](https://www.convex.dev/pricing).

![Página de uso del equipo](/screenshots/teams_usage.png)

Todas las métricas están disponibles en desgloses diarios:

![Gráficos de la página de uso del equipo](/screenshots/teams_usage_2.png)

## Registro de auditoría \{#audit-log\}

<Admonition type="info">
  El registro de auditoría solo está disponible en Convex Professional.
</Admonition>

La [página de registro de auditoría](https://dashboard.convex.dev/team/settings/audit-log) muestra
todas las acciones realizadas por los miembros del equipo. Esto incluye crear y
administrar proyectos y despliegues, invitar y eliminar miembros del equipo y más.

![Página de registro de auditoría del equipo](/screenshots/teams_audit_log.png)

También puedes ver el historial de eventos relacionados con los despliegues en la
[página de historial de despliegues](/dashboard/deployments/history.md).