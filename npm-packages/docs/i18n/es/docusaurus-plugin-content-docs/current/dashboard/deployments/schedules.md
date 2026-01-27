---
title: "Programaciones"
slug: "schedules"
sidebar_position: 30
description:
  "Supervisa y administra funciones programadas y trabajos cron en tu despliegue"
---

La [página de programaciones](https://dashboard.convex.dev/deployment/schedules) muestra
todas las [funciones programadas](/scheduling/scheduled-functions.mdx) y
[trabajos cron](/scheduling/cron-jobs.mdx) en tu despliegue. Usa las pestañas en la
parte superior de esta página para cambiar entre funciones programadas y trabajos cron.

## Interfaz de funciones programadas \{#scheduled-functions-ui\}

La interfaz de funciones programadas muestra una lista de todas las próximas invocaciones de funciones.
Desde aquí, puedes filtrar para ver las ejecuciones programadas de una función específica y cancelar
ejecuciones programadas.

![Funciones programadas](/screenshots/scheduled_functions.png)

## Interfaz de trabajos cron \{#cron-jobs-ui\}

La interfaz de trabajos cron muestra todos tus trabajos cron, incluida la frecuencia con la que se ejecutan y la hora programada de ejecución.

![Cron jobs](/screenshots/cron_jobs.png)

Al expandir un trabajo cron específico se mostrará el historial de ejecución del trabajo seleccionado.

![Cron job history](/screenshots/cron_job_history.png)