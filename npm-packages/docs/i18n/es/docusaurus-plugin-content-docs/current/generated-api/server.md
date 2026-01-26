---
title: "server.js"
sidebar_position: 4
description:
  "Utilidades generadas para implementar consultas, mutaciones y acciones de Convex"
---

<Admonition type="caution" title="Este código se genera automáticamente">
  Estas exportaciones no están disponibles directamente en el paquete `convex`.

  En su lugar, debes ejecutar `npx convex dev` para crear `convex/_generated/server.js`
  y `convex/_generated/server.d.ts`.
</Admonition>

Utilidades generadas para implementar funciones de consulta y mutación de Convex en el servidor.

## Funciones \{#functions\}

### query \{#query\}

▸ **query**(`func`): [`RegisteredQuery`](/api/modules/server#registeredquery)

Define una consulta en la API pública de esta aplicación de Convex.

Esta función tendrá permiso para leer tu base de datos de Convex y será
accesible desde el cliente.

Es un alias de [`queryGeneric`](/api/modules/server#querygeneric) con tipos específicos para el modelo de datos de tu aplicación.

#### Parámetros \{#parameters\}

| Nombre | Descripción                                                                             |
| :----- | :-------------------------------------------------------------------------------------- |
| `func` | La función de consulta. Recibe un [QueryCtx](server.md#queryctx) como primer argumento. |

#### Devuelve \{#returns\}

[`RegisteredQuery`](/api/modules/server#registeredquery)

La consulta encapsulada. Inclúyela como una `exportación` para asignarle un nombre y hacerla accesible.

***

### internalQuery \{#internalquery\}

▸ **internalQuery**(`func`):
[`RegisteredQuery`](/api/modules/server#registeredquery)

Define una consulta que solo es accesible desde otras funciones de Convex (pero
no desde el cliente).

Esta función podrá leer de tu base de datos de Convex. No será accesible desde
el cliente.

Este es un alias de
[`internalQueryGeneric`](/api/modules/server#internalquerygeneric) cuyos tipos
se derivan del modelo de datos de tu aplicación.

#### Parámetros \{#parameters\}

| Nombre | Descripción                                                                            |
| :----- | :------------------------------------------------------------------------------------- |
| `func` | Función de consulta. Recibe un [QueryCtx](server.md#queryctx) como primer argumento. |

#### Devuelve \{#returns\}

[`RegisteredQuery`](/api/modules/server#registeredquery)

La consulta encapsulada. Incluye esta consulta como un `export` para darle un nombre y hacerla accesible.

***

### mutación \{#mutation\}

▸ **mutation**(`func`):
[`RegisteredMutation`](/api/modules/server#registeredmutation)

Define una mutación en la API pública de esta aplicación de Convex.

Esta función podrá modificar la base de datos de Convex y será accesible desde el cliente.

Este es un alias de [`mutationGeneric`](/api/modules/server#mutationgeneric)
con tipos específicos para el modelo de datos de tu aplicación.

#### Parámetros \{#parameters\}

| Nombre | Descripción                                                                             |
| :----- | :-------------------------------------------------------------------------------------- |
| `func` | La función de mutación. Recibe un [MutationCtx](#mutationctx) como su primer argumento. |

#### Devuelve \{#returns\}

[`RegisteredMutation`](/api/modules/server#registeredmutation)

La mutación encapsulada. Inclúyela como un `export` para asignarle un nombre y
hacerla accesible.

***

### internalMutation \{#internalmutation\}

▸ **internalMutation**(`func`):
[`RegisteredMutation`](/api/modules/server#registeredmutation)

Define una mutación que solo es accesible desde otras funciones de Convex (pero
no desde el cliente).

Esta función podrá leer y escribir en tu base de datos de Convex.
No será accesible desde el cliente.

Este es un alias de
[`internalMutationGeneric`](/api/modules/server#internalmutationgeneric) que
está tipado para el modelo de datos de tu aplicación.

#### Parámetros \{#parameters\}

| Nombre | Descripción                                                                                      |
| :----- | :----------------------------------------------------------------------------------------------- |
| `func` | La función de mutación. Recibe un [MutationCtx](server.md#mutationctx) como su primer argumento. |

#### Devuelve \{#returns\}

[`RegisteredMutation`](/api/modules/server#registeredmutation)

La mutación encapsulada. Inclúyela como un `export` para nombrarla y hacerla
accesible.

***

### acción \{#action\}

▸ **action**(`func`): [`RegisteredAction`](/api/modules/server#registeredaction)

Define una acción en la API pública de esta aplicación de Convex.

Una acción es una función que puede ejecutar cualquier código JavaScript, incluido
código no determinista y código con efectos secundarios, como llamar a servicios
de terceros. Puede ejecutarse en el entorno de JavaScript de Convex o en Node.js usando
la directiva `"use node"`. Puede interactuar indirectamente con la base de datos
llamando a consultas y mutaciones mediante [`ActionCtx`](#actionctx).

Este es un alias de [`actionGeneric`](/api/modules/server#actiongeneric) que está
tipado para el modelo de datos de tu aplicación.

#### Parámetros \{#parameters\}

| Nombre | Descripción                                                                        |
| :----- | :--------------------------------------------------------------------------------- |
| `func` | La función de la acción. Recibe un [ActionCtx](#actionctx) como primer argumento. |

#### Devuelve \{#returns\}

[`RegisteredAction`](/api/modules/server#registeredaction)

La función envuelta. Inclúyela como un `export` para asignarle un nombre y hacerla
accesible.

***

### internalAction \{#internalaction\}

▸ **internalAction**(`func`):
[`RegisteredAction`](/api/modules/server#registeredaction)

Define una acción que solo puede invocarse desde otras funciones de Convex (pero no
desde el cliente).

Este es un alias de
[`internalActionGeneric`](/api/modules/server#internalactiongeneric) que está
tipado para el modelo de datos de tu aplicación.

#### Parámetros \{#parameters\}

| Nombre | Descripción                                                                                           |
| :----- | :---------------------------------------------------------------------------------------------------- |
| `func` | La función de acción. Recibe un [ActionCtx](server.md#actionctx) como su primer argumento. |

#### Devuelve \{#returns\}

[`RegisteredAction`](/api/modules/server#registeredaction)

La acción envuelta. Inclúyela como un `export` para asignarle un nombre y hacerla
accesible.

***

### httpAction \{#httpaction\}

▸
**httpAction**(`func: (ctx: ActionCtx, request: Request) => Promise<Response>`):
[`PublicHttpAction`](/api/modules/server#publichttpaction)

#### Parámetros \{#parameters\}

| Name   | Type                                                      | Description                                                                                                                                                                                                                   |
| :----- | :-------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `func` | `(ctx: ActionCtx, request: Request) => Promise<Response>` | La función. Recibe un [`ActionCtx`](/api/modules/server#actionctx) como primer argumento y un [`Request`](https://developer.mozilla.org/en-US/docs/Web/API/Request) como segundo argumento. |

#### Devuelve \{#returns\}

[`PublicHttpAction`](/api/modules/server#publichttpaction)

La función envuelta. Importa esta función desde `convex/http.js` y asígnala a una ruta para conectarla.

## Tipos \{#types\}

### QueryCtx \{#queryctx\}

Ƭ **QueryCtx**: `Object`

Un conjunto de servicios para utilizar dentro de las funciones de consulta de Convex.

El contexto de consulta se pasa como primer argumento a cualquier función de consulta de Convex
que se ejecute en el servidor.

Esto difiere de [MutationCtx](#mutationctx) porque todos los servicios
son de solo lectura.

Este es un alias de [`GenericQueryCtx`](/api/interfaces/server.GenericQueryCtx)
tipado para el modelo de datos de tu aplicación.

#### Declaración de tipo \{#type-declaration\}

| Nombre   | Tipo                                                      |
| :-------- | :--------------------------------------------------------- |
| `db`      | [`DatabaseReader`](#databasereader)                        |
| `auth`    | [`Auth`](/api/interfaces/server.Auth.md)                   |
| `storage` | [`StorageReader`](/api/interfaces/server.StorageReader.md) |
--------------------------------------------------------------------------

### MutationCtx

Ƭ **MutationCtx**: `Object`

Un conjunto de servicios para usar en funciones de mutación de Convex.

El contexto de la mutación se pasa como el primer argumento a cualquier función
de mutación de Convex que se ejecute en el servidor.

Este es un alias de
[`GenericMutationCtx`](/api/interfaces/server.GenericMutationCtx) que está tipado
para el modelo de datos de tu aplicación.

#### Declaración del tipo

| Nombre      | Tipo                                                       |
| :---------- | :--------------------------------------------------------- |
| `db`        | [`DatabaseWriter`](#databasewriter)                        |
| `auth`      | [`Auth`](/api/interfaces/server.Auth.md)                   |
| `storage`   | [`StorageWriter`](/api/interfaces/server.StorageWriter.md) |
| `scheduler` | [`Scheduler`](/api/interfaces/server.Scheduler.md)         |

***

### ActionCtx

Ƭ **ActionCtx**: `Object`

Un conjunto de servicios para usar dentro de funciones de acción de Convex.

El contexto de acción se pasa como el primer argumento a cualquier función de acción de Convex
que se ejecute en el servidor.

Este es un alias de [`ActionCtx`](/api/modules/server#actionctx) con tipos
para el modelo de datos de tu aplicación.

#### Declaración de tipos

| Nombre         | Tipo                                                                                                                                                                         |
| :------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `runQuery`     | (`name`: `string`, `args`?: `Record<string, Value>`) =&gt; `Promise<Value>`                                                                                                     |
| `runMutation`  | (`name`: `string`, `args`?: `Record<string, Value>`) =&gt; `Promise<Value>`                                                                                                     |
| `runAction`    | (`name`: `string`, `args`?: `Record<string, Value>`) =&gt; `Promise<Value>`                                                                                                     |
| `auth`         | [`Auth`](/api/interfaces/server.Auth.md)                                                                                                                                     |
| `scheduler`    | [`Scheduler`](/api/interfaces/server.Scheduler.md)                                                                                                                           |
| `storage`      | [`StorageActionWriter`](/api/interfaces/server.StorageActionWriter.md)                                                                                                       |
| `vectorSearch` | (`tableName`: `string`, `indexName`: `string`, `query`: [`VectorSearchQuery`](/api/interfaces/server.VectorSearchQuery.md)) =&gt; `Promise<Array<{ _id: Id, _score: number }>>` |

***

### DatabaseReader \{#databasewriter\}

Una interfaz para leer de la base de datos dentro de las funciones de consulta de Convex.

Es un alias de
[`GenericDatabaseReader`](/api/interfaces/server.GenericDatabaseReader) que está
tipado para el modelo de datos de tu aplicación.

***

### DatabaseWriter

Una interfaz para leer y escribir en la base de datos dentro de funciones de
mutación de Convex.

Este es un alias de
[`GenericDatabaseWriter`](/api/interfaces/server.GenericDatabaseWriter) con tipos
definidos para el modelo de datos de tu aplicación.