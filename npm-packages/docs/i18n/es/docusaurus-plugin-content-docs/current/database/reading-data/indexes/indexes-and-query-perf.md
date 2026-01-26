---
sidebar_label: "Índices y rendimiento de las consultas"
title: "Introducción a los índices y el rendimiento de las consultas"
sidebar_position: 100
description: "Conoce los efectos de los índices en el rendimiento de las consultas"
---

¿Cómo me aseguro de que mis
[consultas de base de datos](/database/reading-data/reading-data.mdx) en Convex sean rápidas y
eficientes? ¿Cuándo debería definir un
[índice](/database/reading-data/indexes/indexes.md)? ¿Qué es un índice?

Este documento explica cómo debes pensar sobre el rendimiento de las consultas en Convex,
describiendo un modelo simplificado de cómo funcionan las consultas y los índices.

Si ya tienes un conocimiento sólido de las consultas de base de datos y los índices,
puedes pasar directamente a la documentación de referencia:

* [Lectura de datos](/database/reading-data/reading-data.mdx)
* [Índices](/database/reading-data/indexes/indexes.md)

## Una biblioteca de documentos \{#a-library-of-documents\}

Puedes imaginar que Convex es una biblioteca física que almacena documentos como
libros físicos. En este mundo, cada vez que añades un documento a Convex con
[`db.insert("books", {...})`](/api/interfaces/server.GenericDatabaseWriter#insert)
un bibliotecario coloca el libro en un estante.

De forma predeterminada, Convex organiza tus documentos en el orden en que se insertaron. Puedes imaginar al bibliotecario colocando documentos de izquierda a derecha en un estante.

Si ejecutas una consulta para encontrar el primer libro como esta:

```ts
const firstBook = await ctx.db.query("books").first();
```

entonces la bibliotecaria podría empezar en el extremo izquierdo del estante y encontrar el primer
libro. Esta es una consulta extremadamente rápida porque la bibliotecaria solo tiene que mirar
un solo libro para obtener el resultado.

De manera similar, si queremos obtener el último libro que se insertó, en su lugar podríamos
hacer lo siguiente:

```ts
const lastBook = await ctx.db.query("books").order("desc").first();
```

Esta es la misma consulta, pero hemos cambiado el orden a descendente. En la
biblioteca, esto significa que el bibliotecario empezará en el extremo
derecho del estante y revisará de derecha a izquierda. El bibliotecario solo
necesita mirar un único libro para determinar el resultado, por lo que esta
consulta también es extremadamente rápida.

## Escaneos completos de tabla \{#full-table-scans\}

Ahora imagina que alguien llega a la biblioteca y pregunta: &quot;¿Qué libros
de Jane Austen tienen?&quot;

Esto se podría expresar como:

```ts
const books = await ctx.db
  .query("books")
  .filter((q) => q.eq(q.field("author"), "Jane Austen"))
  .collect();
```

Esta consulta dice: &quot;revisa todos los libros, de izquierda a derecha, y recopila
aquellos donde el campo `author` sea Jane Austen&quot;. Para hacer esto, la persona bibliotecaria
tendrá que revisar todo el estante y comprobar quién es el autor de cada libro.

Esta consulta es un *escaneo completo de tabla* porque requiere que Convex examine
cada documento de la tabla. El rendimiento de esta consulta depende de la cantidad
de libros en la biblioteca.

Si tu tabla de Convex tiene una pequeña cantidad de documentos, ¡no hay problema! Los
escaneos completos de tabla deberían seguir siendo rápidos si solo hay unos pocos cientos
de documentos, pero si la tabla tiene muchos miles de documentos estas consultas se volverán
lentas.

En la analogía de la biblioteca, este tipo de consulta está bien si la biblioteca
tiene un único estante. A medida que la biblioteca se expande a una estantería con muchos
estantes o a muchas estanterías, este enfoque se vuelve inviable.

## Catálogos de fichas \{#card-catalogs\}

¿Cómo podemos encontrar libros de un autor de manera más eficiente?

Una opción es reordenar toda la biblioteca por `author`. Esto resolvería
nuestro problema inmediato, pero ahora nuestras consultas originales para
`firstBook` y `lastBook` se convertirían en recorridos completos de la tabla,
porque tendríamos que examinar cada libro para ver cuál se insertó primero o último.

Otra opción es duplicar toda la biblioteca. Podríamos comprar 2 copias de cada
libro y ponerlas en 2 estantes separados: un estante ordenado por tiempo de
inserción y otro ordenado por autor. Esto funcionaría, pero es costoso. Ahora
necesitamos el doble de espacio para nuestra biblioteca.

Una mejor opción es construir un *índice* sobre `author`. En la biblioteca
podríamos usar un [catálogo de fichas](https://en.wikipedia.org/wiki/Library_catalog) de la vieja escuela para
organizar los libros por autor. La idea aquí es que el bibliotecario escribirá
una ficha para cada libro que contenga:

* El autor del libro
* La ubicación del libro en los estantes

Estas fichas se ordenan por autor y van en un archivo separado de los
estantes que contienen los libros. El catálogo de fichas debería mantenerse
pequeño porque solo tiene una ficha por libro (no todo el texto del libro).

![Catálogo de fichas](/img/card-catalog.jpg)

Cuando un usuario pide &quot;libros de Jane Austen&quot;, el bibliotecario ahora puede:

1. Ir al catálogo de fichas y encontrar rápidamente todas las fichas de &quot;Jane Austen&quot;.
2. Para cada ficha, ir y encontrar el libro en el estante.

Esto es bastante rápido porque el bibliotecario puede encontrar con rapidez las
fichas de Jane Austen. Todavía hay un poco de trabajo para encontrar el libro
correspondiente a cada ficha, pero el número de fichas es pequeño, así que esto
es bastante rápido.

## Índices \{#indexes\}

¡Los índices de bases de datos siguen la misma idea! Con Convex puedes definir un
*índice* con:

```ts noDialect title="convex/schema.ts"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  books: defineTable({
    author: v.string(),
    title: v.string(),
    text: v.string(),
  }).index("by_author", ["author"]),
});
```

Entonces Convex creará un nuevo índice llamado `by_author` en `author`. Esto significa
que tu tabla `books` ahora tendrá una estructura de datos adicional que estará
ordenada por el campo `author`.

Puedes consultar este índice con:

```ts
const austenBooks = await ctx.db
  .query("books")
  .withIndex("by_author", (q) => q.eq("author", "Jane Austen"))
  .collect();
```

Esta consulta indica a Convex que vaya al índice `by_author` y encuentre todas las
entradas donde `doc.author === "Jane Austen"`. Como el índice está ordenado por
`author`, esta es una operación muy eficiente. Esto significa que Convex puede ejecutar
esta consulta de la misma manera que el bibliotecario:

1. Encontrar el rango del índice con entradas para Jane Austen.
2. Para cada entrada en ese rango, obtener el documento correspondiente.

El rendimiento de esta consulta depende de la cantidad de documentos donde
`doc.author === "Jane Austen"`, que debería ser bastante pequeña. ¡Hemos acelerado
enormemente la consulta!

## Relleno histórico y mantenimiento de índices \{#backfilling-and-maintaining-indexes\}

Un detalle interesante que considerar es el trabajo necesario para crear esta
nueva estructura. En la biblioteca, la persona bibliotecaria debe revisar cada
libro en el estante y poner una nueva ficha para cada uno en el catálogo de
fichas ordenado por autor. Solo después de eso puede confiar en que el catálogo
de fichas le dará resultados correctos.

¡Lo mismo ocurre con los índices de Convex! Cuando defines un índice nuevo, la
primera vez que ejecutes `npx convex deploy` Convex tendrá que recorrer
todos tus documentos e indexar cada uno. Por eso el primer despliegue después de
la creación de un índice nuevo será un poco más lento de lo normal; Convex tiene
que hacer un poco de trabajo por cada documento de tu tabla. Si la tabla es
particularmente grande, considera usar un
[índice escalonado](/database/reading-data/indexes#staged-indexes) para
completar el relleno histórico de forma asíncrona respecto al despliegue.

De manera similar, incluso después de que se haya definido un índice, Convex
tendrá que hacer un poco de trabajo extra para mantener este índice actualizado
a medida que cambian los datos. Cada vez que se inserta, actualiza o elimina un
documento en una tabla indexada, Convex también actualizará su entrada de
índice. Esto es análogo a que una persona bibliotecaria cree nuevas fichas de
índice para los libros nuevos a medida que los agrega a la biblioteca.

Si estás definiendo unos pocos índices, no necesitas preocuparte por el costo de
mantenimiento. A medida que defines más índices, el costo de mantenerlos crece
porque cada `insert` tiene que actualizar cada índice. Por eso Convex tiene un
límite de 32 índices por tabla. En la práctica, la mayoría de las aplicaciones
definen unos pocos índices por tabla para que sus consultas importantes
sean eficientes.

## Indexar varios campos \{#indexing-multiple-fields\}

Ahora imagina que un lector llega a la biblioteca y quiere sacar
*Foundation* de Isaac Asimov. Gracias a nuestro índice en `author`, podemos escribir una consulta
que use el índice para encontrar todos los libros de Isaac Asimov y luego examine el
título de cada libro para comprobar si es *Foundation*.

```ts
const foundation = await ctx.db
  .query("books")
  .withIndex("by_author", (q) => q.eq("author", "Isaac Asimov"))
  .filter((q) => q.eq(q.field("title"), "Foundation"))
  .unique();
```

Esta consulta describe cómo un bibliotecario podría ejecutar la consulta. El bibliotecario
usará el catálogo de fichas para encontrar todas las fichas de índice de los libros de Isaac Asimov.
Las fichas en sí no tienen el título del libro, así que el bibliotecario necesitará
encontrar cada libro de Asimov en los estantes y mirar su título para encontrar el que
se llama *Foundation*. Por último, esta consulta termina con
[`.unique`](/api/interfaces/server.Query#unique) porque esperamos que haya
como máximo un resultado.

Esta consulta demuestra la diferencia entre filtrar usando
[`withIndex`](/api/interfaces/server.QueryInitializer#withindex) y
[`filter`](/api/interfaces/server.Query#filter). `withIndex` solo te permite
restringir tu consulta en función del índice. Solo puedes hacer operaciones que el
índice pueda hacer de forma eficiente, como encontrar todos los documentos con un autor dado.

`filter`, por otro lado, te permite escribir expresiones arbitrarias y complejas,
pero no se ejecutará usando el índice. En su lugar, las expresiones de `filter` se
evaluarán en cada documento del rango.

Dado todo esto, podemos concluir que **el rendimiento de las consultas indexadas se
basa en cuántos documentos hay en el rango del índice**. En este caso, el
rendimiento se basa en el número de libros de Isaac Asimov porque el bibliotecario
necesitará mirar cada uno para examinar su título.

Desafortunadamente, Isaac Asimov escribió
[muchos libros](https://en.wikipedia.org/wiki/Isaac_Asimov_bibliography_\(alphabetical\)).
En la práctica, incluso con más de 500 libros, esto será lo suficientemente rápido en Convex con el
índice existente, pero consideremos cómo podríamos mejorarlo de todos modos.

Un enfoque es construir un índice `by_title` separado sobre `title`. Esto podría
permitirnos intercambiar el trabajo que hacemos en `.filter` y `.withIndex` para que en su lugar sea:

```ts
const foundation = await ctx.db
  .query("books")
  .withIndex("by_title", (q) => q.eq("title", "Foundation"))
  .filter((q) => q.eq(q.field("author"), "Isaac Asimov"))
  .unique();
```

En esta consulta, aprovechamos eficientemente el índice para encontrar todos los libros llamados
*Foundation* y luego filtramos para encontrar el de Isaac Asimov.

Esto está bien, pero todavía corremos el riesgo de que la consulta sea lenta porque hay demasiados
libros con el título *Foundation*. Un enfoque aún mejor podría ser crear un
índice *compuesto* que indexe tanto `author` como `title`. Los índices compuestos son
índices definidos sobre una lista ordenada de campos.

```ts noDialect title="convex/schema.ts"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  books: defineTable({
    author: v.string(),
    title: v.string(),
    text: v.string(),
  }).index("by_author_title", ["author", "title"]),
});
```

En este índice, los libros se ordenan primero por autor y luego, dentro de cada autor, por título. Esto significa que un bibliotecario puede usar el índice para ir directamente a la sección de Isaac Asimov y encontrar rápidamente *Foundation* allí.

Expresado como una consulta de Convex, se vería así:

```ts
const foundation = await ctx.db
  .query("books")
  .withIndex("by_author_title", (q) =>
    q.eq("author", "Isaac Asimov").eq("title", "Foundation"),
  )
  .unique();
```

Aquí la expresión de rango del índice le indica a Convex que solo considere documentos donde
el autor sea Isaac Asimov y el título sea *Foundation*. Esto corresponde a un solo
documento, así que esta consulta será bastante rápida.

Debido a que este índice ordena por `author` y luego por `title`, también admite eficientemente
consultas como &quot;Todos los libros de Isaac Asimov que empiezan con F.&quot; Podríamos
expresarlo como:

```ts
const asimovBooksStartingWithF = await ctx.db
  .query("books")
  .withIndex("by_author_title", (q) =>
    q.eq("author", "Isaac Asimov").gte("title", "F").lt("title", "G"),
  )
  .collect();
```

Esta consulta usa el índice para encontrar libros donde
`author === "Isaac Asimov" && "F" <= title < "G"`. Una vez más, el rendimiento
de esta consulta depende de cuántos documentos se encuentran en el rango del índice. En este
caso, son solo los libros de Asimov que empiezan con &quot;F&quot;, que es un conjunto bastante pequeño.

Ten en cuenta que este índice también admite nuestra consulta original de &quot;libros de Jane
Austen&quot;. Está bien usar solo el campo `author` en una expresión de rango de índice
y no restringir por título en absoluto.

Por último, imagina que una persona usuaria de la biblioteca pide el libro *The Three-Body Problem*
pero no sabe el nombre de la autora o el autor. Nuestro índice `by_author_title` no nos ayudará
aquí porque primero ordena por `author` y luego por `title`. ¡El título, *The
Three-Body Problem*, podría aparecer en cualquier parte del índice!

Los tipos de TypeScript de Convex en `withIndex` dejan esto claro porque
requieren que compares los campos del índice en orden. Dado que el índice se define en
`["author", "title"]`, primero debes comparar `author` con `.eq` antes que
`title`.

En este caso, la mejor opción probablemente sea crear el índice independiente
`by_title` para facilitar esta consulta.

## Conclusiones \{#conclusions\}

¡Felicidades! Ahora entiendes cómo funcionan las consultas e índices dentro de Convex.

Estos son los puntos principales que hemos cubierto:

1. De forma predeterminada, las consultas de Convex son *escaneos completos de tabla* (*full table scans*). Esto es apropiado para
   crear prototipos y consultar tablas pequeñas.
2. A medida que tus tablas crecen, puedes mejorar el rendimiento de tus consultas añadiendo
   *índices*. Los índices son estructuras de datos separadas que ordenan tus documentos para
   consultas rápidas.
3. En Convex, las consultas usan el método *`withIndex`* para expresar la parte de la
   consulta que usa el índice. El rendimiento de una consulta se basa en cuántos
   documentos hay en la expresión de rango del índice.
4. Convex también admite *índices compuestos* que indexan varios campos.

Para aprender más sobre consultas e índices, consulta nuestra documentación de referencia:

* [Lectura de datos](/database/reading-data/reading-data.mdx)
* [Índices](/database/reading-data/indexes/indexes.md)