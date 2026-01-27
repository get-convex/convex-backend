---
title: "Funciones"
slug: "functions"
sidebar_position: 10
description:
  "Ejecuta, prueba y supervisa funciones de Convex con métricas y datos de rendimiento"
---

![Vista del panel de control de funciones](/screenshots/functions.png)

La [página de funciones](https://dashboard.convex.dev/deployment/functions) muestra
todas las funciones de Convex actualmente desplegadas.

En los despliegues de desarrollo, estas se actualizan continuamente mediante
[`npx convex dev`](/cli.md#run-the-convex-dev-server). Las funciones para
los despliegues de producción se registran con
[`npx convex deploy`](/cli.md#deploy-convex-functions-to-production).

## Ejecutar funciones \{#running-functions\}

Para ejecutar una función de Convex en el panel de control, selecciona una función de la lista en la parte izquierda de la página y haz clic en el botón &quot;Run Function&quot; que aparece junto al nombre de la función.

Si no estás en la página de funciones, todavía puedes abrir esta interfaz mediante el botón persistente *fn* que se muestra en la parte inferior derecha de todas las páginas de despliegue. El atajo de teclado para abrir el ejecutor de funciones es Ctrl + ` (backtick).

Esta vista te permite rellenar los argumentos de tu función y ejecutarla.

Los resultados de las consultas se actualizarán automáticamente a medida que modifiques los argumentos de la función y cambien los datos.

Los resultados de las mutaciones y acciones serán visibles una vez que hagas clic en el botón &quot;Run&quot;.

Ten en cuenta que estos resultados mostrarán los registros y el valor devuelto por la función. Para ver qué cambió cuando ejecutas tu función, consulta la
[página de datos](/dashboard/deployments/data.md).

![Ejecutar una función](/screenshots/run_function.png)

También puedes
[escribir una función de consulta personalizada](/dashboard/deployments/data.md#writing-custom-queries)
eligiendo la opción &quot;Custom test query&quot; en lugar de una de tus funciones desplegadas.

### Consultar una función paginada \{#querying-a-paginated-function\}

Al consultar una función paginada en el panel de control, la interfaz esperará
que los argumentos incluyan
[`PaginationOptions`](/api/interfaces/server.PaginationOptions), es decir, un
objeto que contenga el campo `numItems` y, opcionalmente, el campo `cursor`. El
nombre de este argumento debe ser el mismo que el nombre definido en tu función
de consulta.

* `numItems` debe ser la cantidad de elementos que se incluirán en una página
* `cursor` se puede dejar en blanco para comenzar la paginación. Una vez que
  recibas resultados, puedes asignar a `cursor` el valor del campo
  `continueCursor` del resultado para continuar a la siguiente página.

### Asumir una identidad de usuario \{#assuming-a-user-identity\}

<Admonition type="tip">
  Asumir una identidad de usuario en el panel de control de Convex no te da acceso a
  la identidad real de un usuario. En realidad, este concepto se puede entender como
  &quot;simular&quot; una identidad de usuario dentro de tu función.
</Admonition>

Si estás creando una aplicación autenticada, puede que quieras ejecutar una
función de Convex actuando como una identidad de usuario autenticada.

Para hacerlo, marca la casilla &quot;Act as a user&quot;.

A partir de ahí, puedes escribir en el cuadro que aparece para completar el objeto
de identidad de usuario.

![Acting as a user](/screenshots/acting_as_a_user.png)

Los atributos de usuario válidos son:

| Atributo            | Tipo                                     |
| ------------------- | ---------------------------------------- |
| subject*           | string                                   |
| issuer*            | string                                   |
| name                | string                                   |
| givenName           | string                                   |
| familyName          | string                                   |
| nickname            | string                                   |
| preferredUsername   | string                                   |
| profileUrl          | string                                   |
| email               | string                                   |
| emailVerified       | boolean                                  |
| gender              | string                                   |
| birthday            | string                                   |
| timezone            | string                                   |
| language            | string                                   |
| phoneNumber         | string                                   |
| phoneNumberVerified | boolean                                  |
| address             | string                                   |
| updatedAt           | string (con el formato de una fecha RFC 3339) |
| customClaims        | object                                   |

*Estos atributos son obligatorios.

## Métricas \{#metrics\}

Hay cuatro gráficos básicos para cada función. Para consultar las métricas de uso generales del equipo,
consulta la [configuración del equipo](/dashboard/teams.md#usage).

### Invocaciones \{#invocations\}

Este gráfico muestra la cantidad de veces que se invocó tu función por minuto. A medida que aumenta el uso de tu aplicación, deberías ver que este gráfico también siga una tendencia ascendente.

### Errores \{#errors\}

Un gráfico de todas las excepciones que ocurran mientras se ejecuta tu función. ¿Quieres saber
qué está fallando? Consulta la página de registros, descrita a continuación.

### Tasa de aciertos de caché \{#cache-hit-rate\}

<Admonition type="tip">
  La tasa de aciertos de caché solo se aplica a las funciones de consulta
</Admonition>

Porcentaje que indica con qué frecuencia esta función reutiliza un valor en caché
en lugar de volver a ejecutarse. Tu aplicación tendrá un mejor rendimiento y tus tiempos de respuesta serán
más rápidos cuanto más alta sea la tasa de aciertos de caché.

### Tiempo de ejecución \{#execution-time\}

Cuánto tiempo, en milisegundos, tarda en ejecutarse esta función.

En este gráfico se trazan cuatro líneas individuales: p50, p90, p95 y p99.
Cada una de estas líneas representa el tiempo de respuesta para ese percentil en
la distribución de solicitudes a lo largo del tiempo. Así, solo el 1% de las solicitudes tardó más en ejecutarse que
el tiempo mostrado por la línea de p99. Normalmente, vigilar estas *latencias de cola*
es una buena forma de asegurarte de que tu aplicación obtiene servicios de datos
con rapidez.

Ten en cuenta la relación entre el tiempo de ejecución y la tasa de aciertos de caché. Como
regla general, un acierto de caché tarda bastante menos de 1 ms, así que cuanto mayor sea tu tasa de aciertos de caché,
mejores serán tus tiempos de respuesta.

Al hacer clic en cualquiera de los gráficos obtendrás una vista ampliada y detallada en la que
puedes personalizar los intervalos de tiempo que estás inspeccionando.