---
title: "Estado"
slug: "health"
sidebar_position: 0
description:
  "Supervisa el estado de tu despliegue de Convex, incluidas las tasas de
  errores, el rendimiento de la caché, el estado del planificador y la
  información del despliegue para optimizarlo."
---

La [página de estado](https://dashboard.convex.dev/deployment/) es la página principal
de tu despliegue. En esta página puedes ver información importante sobre
el estado de tu despliegue.

## Tasa de fallos \{#failure-rate\}

![Tarjeta de tasa de fallos](/screenshots/health_failure_rate.png)

La tarjeta de tasa de fallos muestra el porcentaje de solicitudes con error por minuto en la
última hora. La tasa de fallos se calcula como el número de solicitudes con error
dividido por el número total de solicitudes.

## Tasa de aciertos de caché \{#cache-hit-rate\}

![Cache Hit Rate Card](/screenshots/health_cache_hit_rate.png)

La tarjeta “Tasa de aciertos de caché” muestra el porcentaje de aciertos en caché por minuto durante la
última hora. La tasa de aciertos en caché se calcula como el número de aciertos en caché dividido
entre el número total de solicitudes.

La tasa de aciertos de caché solo se aplica a las funciones de consulta.

## Estado del scheduler \{#scheduler-status\}

![Scheduler Status Card](/screenshots/scheduler_overdue.png)

La tarjeta de estado del scheduler muestra el estado del
[scheduler](/scheduling/scheduled-functions). Si el scheduler se retrasa
debido a un exceso de tareas programadas, el estado aparecerá como &quot;Overdue&quot;,
mostrando el tiempo de retraso en minutos.

Haz clic en el botón en la esquina superior derecha de la tarjeta para ver un gráfico
que muestra el estado del scheduler durante la última hora.

![Scheduler Status Chart](/screenshots/scheduler_status.png)

## Último despliegue \{#last-deployed\}

![Tarjeta «Último despliegue»](/screenshots/health_last_deployed.png)

La tarjeta «Último despliegue» muestra la hora a la que se desplegaron tus funciones por última vez.

## Integraciones \{#integrations\}

<Admonition type="info">
  Las integraciones solo están disponibles en Convex Professional.
</Admonition>

![Tarjeta de la última implementación](/screenshots/health_integrations.png)

La tarjeta de integraciones muestra el status de tus integraciones de
[Exception Reporting](/production/integrations/exception-reporting) y
[Log Streams](/production/integrations/log-streams), con accesos directos
para ver y configurar tus integraciones.

## Insights \{#insights\}

![Insights Card](/screenshots/insights.png)

La página Health también muestra insights sobre tu despliegue, con sugerencias
para mejorar el rendimiento y la fiabilidad.

Cada Insight contiene una descripción del problema, el impacto en tu despliegue
(por medio de un gráfico y un registro de eventos) y un enlace para obtener más
información sobre el problema y cómo resolverlo.

Al hacer clic en un Insight se abrirá un desglose del problema, que incluye un
gráfico más grande y una lista de eventos que activaron el Insight.

![Insight Breakdown](/screenshots/insights_breakdown.png)

Los insights disponibles incluyen:

* Funciones que
  [leen demasiados bytes](/production/state/limits#transactions) en una sola
  transacción.
* Funciones que
  [leen demasiados documentos](/production/state/limits#transactions) en una
  sola transacción.
* Funciones que están experimentando [conflictos de escritura](/error#1).