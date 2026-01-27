---
title: "Índices"
sidebar_position: 100
description: "Acelera las consultas con índices de base de datos"
---

Los índices son una estructura de datos que te permite acelerar tus
[consultas de documentos](/database/reading-data/reading-data.mdx#querying-documents)
al indicarle a Convex cómo organizar tus documentos. Los índices también te permiten
cambiar el orden de los documentos en los resultados de las consultas.

Para una introducción más detallada a la indexación, consulta
[Índices y rendimiento de las consultas](/database/reading-data/indexes/indexes-and-query-perf.md).

## Definir índices \{#defining-indexes\}

Los índices se definen como parte de tu [esquema](/database/schemas.mdx) de Convex. Cada
índice se compone de:

1. Un nombre.
   * Debe ser único dentro de la tabla.
2. Una lista ordenada de campos que se indexarán.
   * Para especificar un campo en un documento anidado, usa una ruta separada por puntos como
     `properties.name`.

Para añadir un índice a una tabla, usa el método
[`index`](/api/classes/server.TableDefinition#index) en el esquema de tu tabla:

```ts noDialect title="convex/schema.ts"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

// Define una tabla de mensajes con dos índices.
export default defineSchema({
  messages: defineTable({
    channel: v.id("channels"),
    body: v.string(),
    user: v.id("users"),
  })
    .index("by_channel", ["channel"])
    .index("by_channel_user", ["channel", "user"]),
});
```

El índice `by_channel` está ordenado por el campo `channel` definido en el esquema.
Para los mensajes en el mismo canal, se ordenan por el
[campo `_creationTime` generado por el sistema](/database/types.md#system-fields), que
se agrega automáticamente a todos los índices.

En cambio, el índice `by_channel_user` ordena los mensajes en el mismo `channel`
por el `user` que los envió y solo después por `_creationTime`.

Los índices se crean en [`npx convex dev`](/cli.md#run-the-convex-dev-server) y
[`npx convex deploy`](/cli.md#deploy-convex-functions-to-production).

Es posible que notes que el primer despliegue que define un índice es un poco más lento de lo
normal. Esto se debe a que Convex necesita *backfillear* tu índice. Cuantos más datos haya en
tu tabla, más tiempo le tomará a Convex organizarlos en el orden del índice. Si necesitas
agregar índices a tablas grandes, usa un [índice escalonado](#staged-indexes).

Puedes consultar un índice libremente en el mismo despliegue que lo define. Convex
se asegurará de que el índice esté backfilleado antes de que se registren las nuevas
funciones de consulta y mutación.

<Admonition type="caution" title="Ten cuidado al eliminar índices">
  Además de agregar nuevos índices, `npx convex deploy` eliminará los índices que
  ya no estén presentes en tu esquema. Asegúrate de que tus índices hayan dejado de usarse
  por completo antes de eliminarlos de tu esquema.
</Admonition>

## Consultar documentos mediante índices \{#querying-documents-using-indexes\}

Una consulta para &quot;mensajes en `channel` creados hace 1-2 minutos&quot; sobre el
índice `by_channel` sería:

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel", (q) =>
    q
      .eq("channel", channel)
      .gt("_creationTime", Date.now() - 2 * 60000)
      .lt("_creationTime", Date.now() - 60000),
  )
  .collect();
```

El método [`.withIndex`](/api/interfaces/server.QueryInitializer#withindex)
define qué índice se va a consultar y cómo Convex usará ese índice para
seleccionar documentos. El primer argumento es el nombre del índice y el
segundo es una *expresión de rango de índice*. Una expresión de rango de índice
es una descripción de qué documentos debe considerar Convex al ejecutar la
consulta.

La elección del índice afecta tanto a cómo escribes la expresión de rango de
índice como al orden en el que se devuelven los resultados. Por ejemplo, al
crear tanto un índice `by_channel` como `by_channel_user`, podemos obtener
resultados dentro de un canal ordenados por `_creationTime` o por `user`,
respectivamente. Si fueras a usar el índice `by_channel_user` de esta manera:

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel_user", (q) => q.eq("channel", channel))
  .collect();
```

Los resultados serían todos los mensajes de un `channel` ordenados por `user` y luego por `_creationTime`. Si usaras `by_channel_user` de esta manera:

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel_user", (q) =>
    q.eq("channel", channel).eq("user", user),
  )
  .collect();
```

Los resultados serían los mensajes en el `channel` especificado enviados por `user`, ordenados
por `_creationTime`.

Una expresión de rango de índice siempre consiste en una lista encadenada de:

1. 0 o más expresiones de igualdad definidas con
   [`.eq`](/api/interfaces/server.IndexRangeBuilder#eq).
2. [Opcionalmente] una expresión de límite inferior definida con
   [`.gt`](/api/interfaces/server.IndexRangeBuilder#gt) o
   [`.gte`](/api/interfaces/server.IndexRangeBuilder#gte).
3. [Opcionalmente] una expresión de límite superior definida con
   [`.lt`](/api/interfaces/server.IndexRangeBuilder#lt) o
   [`.lte`](/api/interfaces/server.IndexRangeBuilder#lte).

**Debes recorrer los campos en el orden del índice.**

Cada expresión de igualdad debe comparar un campo de índice diferente, comenzando
desde el principio y en orden. Los límites superior e inferior deben seguir a las
expresiones de igualdad y comparar el siguiente campo.

Por ejemplo, no es posible escribir una consulta como la siguiente:

```ts
// ¡NO COMPILA!
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel", (q) =>
    q
      .gt("_creationTime", Date.now() - 2 * 60000)
      .lt("_creationTime", Date.now() - 60000),
  )
  .collect();
```

Esta consulta no es válida porque el índice `by_channel` está ordenado por
`(channel, _creationTime)` y este rango de consulta compara
`_creationTime` sin antes restringir el rango a un único `channel`.
Como el índice se ordena primero por `channel` y luego por `_creationTime`, no
es un índice útil para encontrar mensajes en todos los canales creados hace
1-2 minutos. Los tipos de TypeScript dentro de `withIndex` te ayudarán con esto.

Para entender mejor qué consultas se pueden ejecutar con qué índices, consulta
[Introducción a los índices y el rendimiento de las consultas](/database/reading-data/indexes/indexes-and-query-perf.md).

**El rendimiento de tu consulta se basa en la especificidad del rango.**

Por ejemplo, si la consulta es

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel", (q) =>
    q
      .eq("channel", channel)
      .gt("_creationTime", Date.now() - 2 * 60000)
      .lt("_creationTime", Date.now() - 60000),
  )
  .collect();
```

entonces el rendimiento de la consulta dependería de la cantidad de mensajes en `channel`
creados hace 1-2 minutos.

Si no se especifica el rango del índice, todos los documentos del índice se
considerarán en la consulta.

<Admonition type="tip" title="Elegir un buen rango de índice">
  Para obtener un buen rendimiento, define rangos de índice tan específicos como
  sea posible. Si estás consultando una tabla grande y no puedes agregar
  condiciones de igualdad con `.eq`, deberías considerar definir un nuevo índice.
</Admonition>

`.withIndex` está diseñado para permitirte especificar únicamente rangos que Convex
pueda usar de forma eficiente con tu índice para encontrar resultados. Para cualquier
otro filtrado, puedes usar el método
[`.filter`](/api/interfaces/server.Query#filter).

Por ejemplo, para consultar &quot;mensajes en `channel` **no** creados por mí&quot; podrías
hacer lo siguiente:

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel", (q) => q.eq("channel", channel))
  .filter((q) => q.neq(q.field("user"), myUserId))
  .collect();
```

En este caso, el rendimiento de esta consulta se basará en cuántos mensajes
haya en el canal. Convex tendrá en cuenta cada mensaje del canal y solo
devolverá los mensajes en los que el campo `user` no coincida con `myUserId`.

## Ordenar con índices \{#sorting-with-indexes\}

Las consultas que usan `withIndex` se ordenan según las columnas especificadas en el índice.

El orden de las columnas en el índice determina la prioridad de ordenación. Los
valores de las columnas que aparecen primero en el índice se comparan primero. Las columnas
siguientes solo se comparan como criterio de desempate y únicamente si todas las columnas anteriores coinciden.

Dado que Convex incluye automáticamente `_creationTime` como la última columna en todos
los índices, `_creationTime` siempre será el criterio de desempate final si todas las demás
columnas del índice son iguales.

Por ejemplo, `by_channel_user` incluye `channel`, `user` y `_creationTime`.
Entonces, las consultas sobre `messages` que usan `.withIndex("by_channel_user")` se ordenarán
primero por canal, luego por usuario dentro de cada canal y, finalmente, por la hora de creación.

Ordenar con índices te permite cubrir casos de uso como mostrar a los `N` usuarios
con mayor puntuación, las `N` transacciones más recientes o los `N` mensajes con más &quot;me gusta&quot;.

Por ejemplo, para obtener a los 10 jugadores con mayor puntuación en tu juego, podrías
definir un índice sobre la puntuación más alta del jugador:

```ts
export default defineSchema({
  players: defineTable({
    username: v.string(),
    highestScore: v.number(),
  }).index("by_highest_score", ["highestScore"]),
});
```

Entonces puedes encontrar de forma eficiente a los 10 jugadores con la puntuación más alta usando tu
índice y [`take(10)`](/api/interfaces/server.Query#take):

```ts
const topScoringPlayers = await ctx.db
  .query("users")
  .withIndex("by_highest_score")
  .order("desc")
  .take(10);
```

En este ejemplo, se omite la expresión de rango porque estamos buscando a los
jugadores con mayor puntuación de todos los tiempos. Esta consulta en
particular es razonablemente eficiente para conjuntos de datos grandes
únicamente porque usamos `take()`.

Si usas un índice sin una expresión de rango, siempre debes usar uno de los
siguientes junto con `withIndex`:

1. [`.first()`](/api/interfaces/server.Query#first)
2. [`.unique()`](/api/interfaces/server.Query#unique)
3. [`.take(n)`](/api/interfaces/server.Query#take)
4. [`.paginate(ops)`](/database/pagination.mdx)

Estas API te permiten limitar tu consulta de forma eficiente a un tamaño
razonable sin realizar un escaneo completo de la tabla.

<Admonition type="caution" title="Escaneos completos de tabla">
  Cuando tu consulta recupera documentos de la base de datos, escaneará las filas
  en el rango que especifiques. Si estás usando `.collect()`, por ejemplo,
  escaneará todas las filas del rango. Así que si usas `withIndex` sin una
  expresión de rango, estarás
  [escaneando la tabla completa](https://docs.convex.dev/database/indexes/indexes-and-query-perf#full-table-scans),
  lo cual puede ser lento cuando tu tabla tiene miles de filas. `.filter()` no
  afecta qué documentos se escanean. Usar `.first()`, `.unique()` o
  `.take(n)` solo escaneará filas hasta que tenga suficientes documentos.
</Admonition>

Puedes incluir una expresión de rango para satisfacer consultas más
específicas. Por ejemplo, para obtener a los jugadores con mayor puntuación en
Canadá, podrías usar tanto `take()` como una expresión de rango:

```ts
// consulta los 10 jugadores con puntuación más alta en Canadá.
const topScoringPlayers = await ctx.db
  .query("users")
  .withIndex("by_country_highest_score", (q) => q.eq("country", "CA"))
  .order("desc")
  .take(10);
```

## Índices por etapas \{#staged-indexes\}

De forma predeterminada, la creación de índices ocurre de manera síncrona cuando haces un deploy del código. Para tablas grandes, el proceso de
[rellenar el índice](indexes-and-query-perf#backfilling-and-maintaining-indexes)
de la tabla existente puede ser lento. Los índices por etapas son una forma de crear un índice
en una tabla grande de forma asíncrona sin bloquear el deploy. Esto puede ser útil si
estás trabajando en varias funcionalidades a la vez.

Para crear un índice por etapas, usa la siguiente sintaxis en tu archivo `schema.ts`.

```ts
export default defineSchema({
  messages: defineTable({
    channel: v.id("channels"),
  }).index("by_channel", { fields: ["channel"], staged: true }),
});
```

<Admonition type="caution" title="Los índices en preparación no se pueden usar hasta que estén habilitados">
  Los índices en preparación no se pueden usar en consultas hasta que los habilites. Para habilitarlos,
  primero debe completarse el proceso de backfill.
</Admonition>

Puedes comprobar el progreso del backfill en el
panel [*Indexes*](/dashboard/deployments/data/#view-the-indexes-of-a-table) de
la página de datos del panel de control. Una vez que se complete, puedes habilitar el índice y empezar a usarlo quitando la opción `staged`.

```ts
export default defineSchema({
  messages: defineTable({
    channel: v.id("channels"),
  }).index("by_channel", { fields: ["channel"] }),
});
```

## Límites \{#limits\}

Convex admite índices que contienen hasta 16 campos. Puedes definir hasta 32 índices en
cada tabla. Los índices no pueden contener campos duplicados.

No se permiten campos reservados (que comiencen con `_`) en los índices. El
campo `_creationTime` se agrega automáticamente al final de cada índice para garantizar
un orden estable. No debe agregarse explícitamente en la definición del índice
y cuenta para el límite de campos del índice.

El índice `by_creation_time` se crea automáticamente (y es el que se usa en
las consultas de base de datos que no especifican un índice). El índice `by_id` está reservado.