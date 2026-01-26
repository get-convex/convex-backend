---
title: "Tipos de datos"
sidebar_position: 40
description: "Tipos de datos admitidos en documentos de Convex"
---

import ConvexValues from "@site/i18n/es/docusaurus-plugin-content-docs/current/\_convexValues.mdx";

Todos los documentos de Convex se definen como objetos de JavaScript. Estos objetos pueden tener
valores de campo de cualquiera de los tipos siguientes.

Puedes definir la estructura de los documentos dentro de tus tablas
[definiendo un esquema](/database/schemas.mdx).

## Valores de Convex \{#convex-values\}

<ConvexValues />

## Campos del sistema \{#system-fields\}

Cada documento en Convex tiene dos campos del sistema generados automáticamente:

* `_id`: El [identificador del documento](/database/document-ids.mdx).
* `_creationTime`: El momento en que se creó este documento, en milisegundos desde la época Unix.

## Límites \{#limits\}

Los valores de Convex deben ser menores de 1 MB en tamaño total. Este es un límite
aproximado por ahora, pero si te estás encontrando con estos límites y quieres un método
más preciso para calcular el tamaño de un documento,
[ponte en contacto con nosotros](https://convex.dev/community). Los documentos pueden tener valores
anidados, ya sean objetos o arrays que contengan otros tipos de Convex. Los tipos de Convex
pueden tener como máximo 16 niveles de anidación, y el tamaño acumulado de un árbol anidado
de valores debe estar por debajo del límite de 1 MB.

Los nombres de tablas pueden contener caracteres alfanuméricos (&quot;a&quot; a &quot;z&quot;, &quot;A&quot; a &quot;Z&quot; y &quot;0&quot;
a &quot;9&quot;) y guiones bajos (&quot;&#95;&quot;), y no pueden comenzar con un guion bajo.

Para obtener información sobre otros límites, consulta [aquí](/production/state/limits.mdx).

Si alguno de estos límites no funciona para ti,
[háznoslo saber](https://convex.dev/community)!

## Trabajar con `undefined` \{#working-with-undefined\}

El valor de TypeScript `undefined` no es un valor válido de Convex, por lo que no se puede
usar en argumentos ni valores de retorno de funciones de Convex, ni en documentos almacenados.

1. Los objetos/registros con valores `undefined` son equivalentes a que el campo no
   exista: `{a: undefined}` se transforma en `{}` cuando se pasa a una función
   o se almacena en la base de datos. Puedes pensar en las llamadas a funciones de Convex y en
   la base de datos de Convex como si serializaran los datos con `JSON.stringify`, que
   de forma similar elimina los valores `undefined`.
2. Los validadores para campos de objetos pueden usar `v.optional(...)` para indicar que el
   campo podría no estar presente.
   * Si falta el campo `"a"` de un objeto, es decir, `const obj = {};`, entonces
     `obj.a === undefined`. Esta es una propiedad de TypeScript/JavaScript, no
     algo específico de Convex.
3. Puedes usar `undefined` en filtros y en consultas sobre índices, y coincidirá con
   documentos que no tengan el campo. Es decir,
   `.withIndex("by_a", q=>q.eq("a", undefined))` coincide con los documentos `{}` y
   `{b: 1}`, pero no con `{a: 1}` ni con `{a: null, b: 1}`.
   * En el esquema de ordenación de Convex, `undefined < null < todos los demás valores`, por lo que
     puedes hacer coincidir documentos que *tienen* un campo usando `q.gte("a", null as any)` o
     `q.gt("a", undefined)`.
4. Hay exactamente un caso en el que `{a: undefined}` es diferente de `{}`: cuando se
   pasa a `ctx.db.patch`. Pasar `{a: undefined}` elimina el campo `"a"` del
   documento, mientras que pasar `{}` no cambia el campo `"a"`. Consulta
   [Actualizar documentos existentes](/database/writing-data.mdx#updating-existing-documents).
5. Dado que `undefined` se elimina de los argumentos de función pero tiene significado en
   `ctx.db.patch`, hay algunos trucos para pasar el argumento de `patch` desde el
   cliente.
   * Si el cliente pasa `args={}` (o `args={a: undefined}`, que es
     equivalente) y debería dejar el campo `"a"` sin cambios, usa
     `ctx.db.patch(id, args)`.
   * Si el cliente pasa `args={}` y esto debería eliminar el campo `"a"`, usa
     `ctx.db.patch(id, {a: undefined, ...args})`.
   * Si el cliente pasando `args={}` debería dejar el campo `"a"` sin cambios y
     `args={a: null}` debería eliminarlo, podrías hacer:
     ```ts
     if (args.a === null) {
       args.a = undefined;
     }
     await ctx.db.patch(tableName, id, args);
     ```
6. Las funciones que retornan un `undefined`/`void` simple se tratan como si
   devolvieran `null`.
7. Los arrays que contienen valores `undefined`, como `[undefined]`, lanzan un error cuando
   se usan como valores de Convex.

Si prefieres evitar los comportamientos especiales de `undefined`, puedes usar
`null` en su lugar, que *sí* es un valor válido de Convex.

## Trabajar con fechas y horas \{#working-with-dates-and-times\}

Convex no tiene un tipo de dato específico para trabajar con fechas y horas. Cómo
almacenes las fechas depende de las necesidades de tu aplicación:

1. Si solo te importa un instante concreto, puedes almacenar una
   [marca de tiempo UTC](https://en.wikipedia.org/wiki/Unix_time). Recomendamos
   seguir el ejemplo del campo `_creationTime`, que almacena la marca de tiempo
   como un `number` en milisegundos. En tus funciones y en el cliente puedes crear
   un objeto `Date` de JavaScript pasando la marca de tiempo a su constructor:
   `new Date(timeInMsSinceEpoch)`. Luego puedes imprimir la fecha y la hora en la
   zona horaria deseada (como la zona horaria configurada en la máquina de tu
   usuario).
   * Para obtener la marca de tiempo UTC actual en tu función y almacenarla en la
     base de datos, usa `Date.now()`
2. Si te importa una fecha de calendario o una hora específica, como al
   implementar una aplicación de reservas, deberías almacenar la fecha y/o la hora
   exactas como una cadena. Si tu aplicación admite varias zonas horarias también
   deberías almacenar la zona horaria. [ISO8601](https://en.wikipedia.org/wiki/ISO_8601)
   es un formato habitual para almacenar fechas y horas juntas en una sola cadena,
   como `"2024-03-21T14:37:15Z"`. Si tus usuarios pueden elegir una zona horaria
   específica probablemente deberías almacenarla en un campo `string` separado,
   normalmente usando el
   [nombre de zona horaria IANA](https://en.wikipedia.org/wiki/Tz_database#Names_of_time_zones)
   (aunque podrías concatenar los dos campos con un carácter único como
   `"|"`).

Para un formateo y una manipulación más sofisticados de fechas y horas,
usa una de las bibliotecas populares de JavaScript: [date-fns](https://date-fns.org/),
[Day.js](https://day.js.org/), [Luxon](https://moment.github.io/luxon/) o
[Moment.js](https://momentjs.com/).