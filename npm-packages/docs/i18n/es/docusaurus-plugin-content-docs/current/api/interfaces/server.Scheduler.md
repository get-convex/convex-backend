---
id: "server.Scheduler"
title: "Interfaz: Scheduler"
custom_edit_url: null
---

[server](../modules/server.md).Scheduler

Una interfaz para programar funciones de Convex.

Puedes programar mutaciones o acciones. Las mutaciones tienen la garantía de ejecutarse
exactamente una vez: se reintentan automáticamente ante errores transitorios y o bien se ejecutan
correctamente o fallan de manera determinista debido a un error del desarrollador al definir la
función. Las acciones se ejecutan como máximo una vez: no se reintentan y pueden fallar
debido a errores transitorios.

Considera usar un internalMutation o internalAction para asegurar que
estas funciones no puedan llamarse directamente desde un cliente de Convex.

## Métodos \{#methods\}

### runAfter \{#runafter\}

▸ **runAfter**&lt;`FuncRef`&gt;(`delayMs`, `functionReference`, `...args`): `Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_scheduled_functions"`&gt;&gt;

Programa una función para que se ejecute tras un intervalo de tiempo.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `FuncRef` | extiende [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `delayMs` | `number` | Retardo en milisegundos. Debe ser un valor no negativo. Si el retardo es cero, la función programada quedará lista para ejecutarse inmediatamente después de que finalice la función que la programó. |
| `functionReference` | `FuncRef` | Un [FunctionReference](../modules/server.md#functionreference) para la función que se va a programar. |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | Argumentos con los que se llamará a la función programada. |

#### Devuelve \{#returns\}

`Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_scheduled_functions"`&gt;&gt;

#### Definido en \{#defined-in\}

[server/scheduler.ts:41](https://github.com/get-convex/convex-js/blob/main/src/server/scheduler.ts#L41)

***

### runAt \{#runat\}

▸ **runAt**&lt;`FuncRef`&gt;(`timestamp`, `functionReference`, `...args`): `Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_scheduled_functions"`&gt;&gt;

Programa una función para que se ejecute en un momento (timestamp) determinado.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `FuncRef` | extiende [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `timestamp` | `number` | `Date` | Un `Date` o un timestamp (milisegundos desde la época Unix). Si el `timestamp` está en el pasado, la función programada se ejecutará inmediatamente después de que finalice la función que la programa. El `timestamp` no puede estar más de cinco años en el pasado ni más de cinco años en el futuro. |
| `functionReference` | `FuncRef` | Un [FunctionReference](../modules/server.md#functionreference) de la función que se va a programar. |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | argumentos con los que llamar a las funciones programadas. |

#### Devuelve \{#returns\}

`Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_scheduled_functions"`&gt;&gt;

#### Definido en \{#defined-in\}

[server/scheduler.ts:58](https://github.com/get-convex/convex-js/blob/main/src/server/scheduler.ts#L58)

***

### cancel \{#cancel\}

▸ **cancel**(`id`): `Promise`&lt;`void`&gt;

Cancela una función previamente programada si aún no ha comenzado. Si la
función programada ya está en progreso, continuará ejecutándose, pero
cualquier función nueva que intente programar quedará cancelada.

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`"_scheduled_functions"`&gt; |

#### Devuelve \{#returns\}

`Promise`&lt;`void`&gt;

#### Definido en \{#defined-in\}

[server/scheduler.ts:71](https://github.com/get-convex/convex-js/blob/main/src/server/scheduler.ts#L71)