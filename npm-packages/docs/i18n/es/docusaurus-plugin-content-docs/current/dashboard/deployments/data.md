---
title: "Datos"
slug: "data"
sidebar_position: 5
description:
  "Ver, editar y gestionar tablas y documentos de la base de datos en el panel de control"
---

![Página de panel de control de datos](/screenshots/data.png)

La [página de datos](https://dashboard.convex.dev/deployment/data) te permite ver
y gestionar todas tus tablas y documentos.

En la parte izquierda de la página hay una lista de tus tablas. Al hacer clic en una tabla
puedes crear, ver, actualizar y eliminar documentos en esa tabla.

Puedes arrastrar y soltar los encabezados de columna en cada tabla para reordenar
visualmente los datos.

Hay una vista de solo lectura de la página de datos disponible en la
[interfaz de línea de comandos](/cli.md#display-data-from-tables).

```sh
npx convex data [table]
```

## Filtrar documentos \{#filtering-documents\}

Puedes filtrar documentos en la página de datos haciendo clic en el botón &quot;Filter&quot;
en la parte superior de la página.

![Filtros de datos](/screenshots/data_filters.png)

Todos los campos de un documento se pueden filtrar usando las operaciones
permitidas en la sintaxis de consultas de Convex. [Equality](/database/reading-data/filters.mdx#equality-conditions)
y [comparisons](/database/reading-data/filters.mdx#comparisons) comparten las
mismas reglas al filtrar en el panel de control que una consulta usando el
cliente de Convex. También puedes filtrar en función del tipo del campo.

Para añadir un filtro, haz clic en `+` junto a un filtro existente. Si añades
más de una condición, se evaluarán usando la operación `and`.

Para cada filtro, debes seleccionar un campo por el que filtrar, la operación y
un valor de comparación. En la tercera casilla de entrada (al seleccionar un
valor), puedes introducir un valor válido de Convex, como `"a string"`, `123` o
incluso un objeto complejo, como `{ a: { b: 2 } }`

<Admonition type="note">
  Al filtrar por `_creationTime`, se mostrará un selector de fecha en lugar del
  campo de entrada normal de sintaxis de JavaScript. Las comparaciones para
  `_creationTime` se realizan con granularidad de nanosegundos, así que si deseas
  filtrar a un momento exacto, intenta añadir dos condiciones de filtro para
  `creationTime >= $time` y `creationTime <= $time + 1 minute`.
</Admonition>

## Escribir consultas personalizadas \{#writing-custom-queries\}

Puedes escribir una [consulta](/database/reading-data/reading-data.mdx) directamente en el
panel de control. Esto te permite realizar filtrado y transformación arbitrarios de
los datos, incluyendo ordenación, joins, agrupación y agregación.

En el menú `⋮` de desbordamiento en la parte superior de la página de datos, haz clic en la opción “Custom query”.

<img src="/screenshots/data_custom_query.png" alt="Botón de consulta personalizada" width={250} />

Esto abre la misma interfaz de usuario que se usa para
[ejecutar tus funciones desplegadas](/dashboard/deployments/functions.md#running-functions),
pero con la opción “Custom test query” seleccionada, lo que te permite editar el
código fuente de la consulta. Este código fuente se enviará a tu despliegue y
se ejecutará cuando hagas clic en el botón “Run Custom Query”.

![Ejecución de una consulta de prueba personalizada](/screenshots/data_custom_query_runner.png)

Si no estás en la página de datos, aún puedes abrir esta interfaz mediante el botón
*fn* persistente que se muestra en la parte inferior derecha de todas las páginas de despliegue. El atajo de teclado para abrir el ejecutor de funciones es Ctrl + ` (tecla de acento grave/backtick).

## Creación de tablas \{#creating-tables\}

Puedes crear una tabla desde el panel de control haciendo clic en el botón «Create Table»
y escribiendo un nombre para la nueva tabla.

## Creación de documentos \{#creating-documents\}

Puedes añadir documentos individuales a la tabla usando el botón “Add Documents”
ubicado en la barra de herramientas de la tabla de datos.

Una vez que hagas clic en “Add Documents” se abrirá un panel lateral que te
permitirá añadir nuevos documentos a tu tabla usando sintaxis de JavaScript. Para
añadir más de un documento a la vez, añade nuevos objetos al array en el editor.

![Add document](/screenshots/data_add_document.png)

## Acciones rápidas (menú contextual) \{#quick-actions-context-menu\}

Puedes hacer clic con el botón derecho en un documento o valor para abrir un menú contextual con
acciones rápidas, como copiar valores, filtrar rápidamente por el valor seleccionado y
eliminar documentos.

![Menú contextual de acciones rápidas](/screenshots/data_context_menu.png)

## Editar una celda \{#editing-a-cell\}

Para editar el valor de una celda, haz doble clic en la celda de la tabla de datos o presiona la tecla Enter mientras está seleccionada. Puedes cambiar la celda seleccionada usando las teclas de flecha.

Puedes cambiar el valor editándolo en línea y presionando la tecla Enter para guardar.

<Admonition type="note">
  Incluso puedes editar el tipo de tu valor aquí, siempre que cumpla con tu
  [esquema](/database/schemas.mdx); prueba a sustituir una cadena de texto por un objeto.
</Admonition>

![Editor de valores en línea](/screenshots/data_edit_inline.png)

## Editar un documento \{#editing-a-document\}

Para editar varios campos de un documento al mismo tiempo, pasa el cursor sobre el documento
y haz clic con el botón derecho para abrir el menú contextual. Desde allí puedes hacer clic en &quot;Edit Document&quot;.

![Editar documento completo](/screenshots/data_edit_document.png)

## Agregar referencias a otros documentos \{#adding-references-to-other-documents\}

Para hacer referencia a otro documento, utiliza el Id en forma de cadena del documento al que quieres hacer referencia.

Puedes copiar el Id haciendo clic en su celda y presionando CTRL/CMD+C.

## Edición masiva de documentos \{#bulk-editing-documents\}

Puedes editar varios documentos o todos a la vez. Para seleccionar todos los documentos, haz clic en la casilla de verificación de la fila de encabezado de la tabla. Para seleccionar documentos individuales, pasa el cursor sobre la celda más a la izquierda y haz clic en la casilla de verificación que aparece. Para seleccionar varios documentos adyacentes a la vez, mantén pulsada la tecla Shift mientras haces clic en la casilla de verificación.

Cuando haya al menos un documento seleccionado, el botón “(Bulk) Edit Document(s)” será visible en la barra de herramientas de la tabla. Haz clic en el botón y aparecerá un editor en el lado derecho.

![Bulk edit documents](/screenshots/data_bulk_edit.png)

## Eliminación de documentos \{#deleting-documents\}

Cuando haya al menos un documento seleccionado (ver arriba), el botón “Delete Document(s)”
será visible en la barra de herramientas de la tabla. Haz clic en el botón para eliminar
documentos. Si estás modificando datos en un despliegue de producción, aparecerá un cuadro de diálogo
de confirmación antes de que se eliminen los documentos.

## Vaciar una tabla \{#clear-a-table\}

También puedes eliminar todos los documentos haciendo clic en el menú de opciones `⋮` en la parte superior de la página de datos y luego haciendo clic en &quot;Clear Table&quot;. Esta acción eliminará todos los documentos de la tabla, sin eliminar la tabla en sí.

En entornos de producción, el panel de control de Convex te pedirá que introduzcas el nombre de la tabla antes de eliminarla.

## Eliminar una tabla \{#delete-a-table\}

<Admonition type="caution" title="Esta es una acción permanente">
  Eliminar una tabla es irreversible. En entornos de producción, el panel de control
  de Convex te pedirá que escribas el nombre de la tabla antes de eliminarla.
</Admonition>

El botón &quot;Delete table&quot; se encuentra haciendo clic en el menú de opciones `⋮` en
la parte superior de la página de datos. Esta acción eliminará todos los documentos de esta tabla
y quitará la tabla de tu lista de tablas. Si esta tabla tenía índices, tendrás
que volver a desplegar tus funciones de Convex (ejecutando `npx convex deploy` o
`npx convex dev` para producción o desarrollo, respectivamente) para recrear los
índices.

## Generar un esquema \{#generating-a-schema\}

En la esquina inferior izquierda de la página hay un botón «Generate Schema» en el que puedes hacer clic para que Convex genere un [esquema](/database/schemas.mdx) de todos tus documentos dentro de esta tabla.

![Botón «Generate Schema»](/screenshots/data_generate_schema.png)

## Ver el esquema de una tabla \{#view-the-schema-of-a-table\}

El botón &quot;Schema&quot; se encuentra haciendo clic en el menú de opciones `⋮` en la parte superior
de la página de datos.

Este botón abrirá un panel que muestra los
[esquemas](/database/schemas.mdx) guardados y generados asociados con la tabla seleccionada.

## Ver los índices de una tabla \{#view-the-indexes-of-a-table\}

El botón &quot;Indexes&quot; se encuentra al hacer clic en el menú `⋮` en la parte
superior de la página de datos.

Este botón abrirá un panel que muestra los
[índices](/database/reading-data/indexes/indexes.md) asociados a la tabla
seleccionada.

Los índices que aún no hayan completado el proceso de backfill irán acompañados de un
icono de carga junto a su nombre.