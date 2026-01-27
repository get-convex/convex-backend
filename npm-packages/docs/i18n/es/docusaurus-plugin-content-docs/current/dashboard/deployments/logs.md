---
title: "Registros"
slug: "logs"
sidebar_position: 40
description:
  "Consulta registros de funciones en tiempo real y la actividad del despliegue en tu panel de control"
---

![Logs Dashboard Page](/screenshots/logs.png)

La [página de registros](https://dashboard.convex.dev/deployment/logs) es una vista en tiempo real
de toda la actividad que ocurre dentro de tu despliegue.

La página de registros proporciona un historial corto de registros de funciones recientes y mostrará
nuevos registros a medida que se generen. Para almacenar un historial más largo de registros, puedes
configurar un [flujo de registros](/production/integrations/log-streams/log-streams.mdx).

La actividad de funciones incluye:

* La hora de ejecución de la función.
* El ID de solicitud de la ejecución de la función.
* El resultado de la ejecución de la función (éxito o fallo).
* El nombre de la función invocada.
* La salida de la función, incluida cualquier línea de registro generada por la función (por ej.
  `console.log`) y las excepciones.
* La duración de la ejecución de la función, en milisegundos (no incluye la latencia
  de red).

Además de la actividad de funciones,
[los eventos del despliegue](/dashboard/deployments/history.md) que describen cambios de configuración
aparecerán aquí.

Al hacer clic en un registro se abrirá una vista con todos los registros asociados con el mismo ID de
solicitud que el registro seleccionado. Esto puede ser útil para depurar errores y
entender el contexto de la ejecución de una función.

![Request ID Logs](/screenshots/request_logs.png)

Puedes usar los controles en la parte superior de esta página para filtrar registros por texto, nombre
de función, estado de ejecución y nivel de gravedad del registro.

### Filtrar registros \{#filter-logs\}

Usa el cuadro de texto &quot;Filter logs...&quot; en la parte superior de la página para filtrar el texto de los registros.

Puedes usar la lista desplegable “Functions” para incluir o excluir funciones de
los resultados.

También puedes encontrar registros para un error específico usando &quot;Filter logs&quot; y el
[Convex request id](/functions/error-handling/error-handling.mdx#debugging-errors).
Por ejemplo, si ves este `Error` en la consola de tu navegador:

![Browser Error](/screenshots/console_error_requestid.png)

Puedes ver los registros de esa función en tu panel de control pegando ese
Request ID en la barra de búsqueda &#39;Search logs...&#39; en la página
[Logs](/dashboard/deployments/logs.md) del panel de control de Convex. Ten en cuenta
que como esta página no es una vista histórica completa de los registros, es posible que no
encuentres registros de solicitudes antiguas.

La mayoría de los servicios de error reporting y destinos de logs también deberían poder buscarse por Request
ID.

### Tipos de registros \{#log-types\}

Los registros también se pueden filtrar por tipo. Los tipos incluyen resultados de funciones (éxito o fallo) y niveles de gravedad (info, warn, debug, error).

Todas las ejecuciones fallidas incluirán una causa, que por lo general será una excepción de JavaScript.