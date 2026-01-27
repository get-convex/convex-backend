---
id: "server.Crons"
title: "Clase: Crons"
custom_edit_url: null
---

[server](../modules/server.md).Crons

Una clase para programar tareas cron.

Para más información, consulta la documentación en https://docs.convex.dev/scheduling/cron-jobs

## Constructores \{#constructors\}

### constructor \{#constructor\}

• **new Crons**()

#### Definido en \{#defined-in\}

[server/cron.ts:246](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L246)

## Propiedades \{#properties\}

### crons \{#crons\}

• **crons**: `Record`&lt;`string`, [`CronJob`](../interfaces/server.CronJob.md)&gt;

#### Definido en \{#defined-in\}

[server/cron.ts:244](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L244)

***

### isCrons \{#iscrons\}

• **isCrons**: `true`

#### Definido en \{#defined-in\}

[server/cron.ts:245](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L245)

## Métodos \{#methods\}

### interval \{#interval\}

▸ **interval**&lt;`FuncRef`&gt;(`cronIdentifier`, `schedule`, `functionReference`, `...args`): `void`

Programa una mutación o acción para que se ejecute periódicamente.

```js
crons.interval("Clear presence data", {seconds: 30}, api.presence.clear);
```

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `FuncRef` | se extiende de [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | - |
| `schedule` | `Interval` | El tiempo entre ejecuciones de este trabajo programado. |
| `functionReference` | `FuncRef` | Un [FunctionReference](../modules/server.md#functionreference) para programar la función. |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | Los argumentos de la función. |

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[server/cron.ts:283](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L283)

***

### hourly \{#hourly\}

▸ **hourly**&lt;`FuncRef`&gt;(`cronIdentifier`, `schedule`, `functionReference`, `...args`): `void`

Programa una mutación o acción para que se ejecute cada hora.

```js
crons.hourly(
  "Reset high scores",
  {
    minuteUTC: 30,
  },
  api.scores.reset
)
```

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `FuncRef` | extiende [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### Parámetros \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | Un nombre único para esta tarea programada. |
| `schedule` | `Hourly` | A qué hora (UTC) de cada día se debe ejecutar esta función. |
| `functionReference` | `FuncRef` | Una [FunctionReference](../modules/server.md#functionreference) para la función a programar. |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | Los argumentos de la función. |

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[server/cron.ts:331](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L331)

***

### daily \{#daily\}

▸ **daily**&lt;`FuncRef`&gt;(`cronIdentifier`, `schedule`, `functionReference`, `...args`): `void`

Programa una mutación o acción para que se ejecute a diario.

```js
crons.daily(
  "Reset high scores",
  {
    hourUTC: 17, // (9:30am Pacífico/10:30am Pacífico con horario de verano)
    minuteUTC: 30,
  },
  api.scores.reset
)
```

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `FuncRef` | extiende [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### Parámetros \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | Un nombre único para esta tarea programada. |
| `schedule` | `Daily` | A qué hora (UTC) ejecutar esta función cada día. |
| `functionReference` | `FuncRef` | Una [FunctionReference](../modules/server.md#functionreference) de la función que se va a programar. |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | Los argumentos de la función. |

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[server/cron.ts:366](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L366)

***

### weekly \{#weekly\}

▸ **weekly**&lt;`FuncRef`&gt;(`cronIdentifier`, `schedule`, `functionReference`, `...args`): `void`

Programa una mutación o acción para que se ejecute semanalmente.

```js
crons.weekly(
  "Weekly re-engagement email",
  {
    dayOfWeek: "Tuesday",
    hourUTC: 17, // (9:30am Pacífico/10:30am Pacífico con horario de verano)
    minuteUTC: 30,
  },
  api.emails.send
)
```

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `FuncRef` | extends [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | Un nombre único para esta tarea programada. |
| `schedule` | `Weekly` | Qué día y a qué hora (UTC) de cada semana se ejecutará esta función. |
| `functionReference` | `FuncRef` | Un [FunctionReference](../modules/server.md#functionreference) de la función que se va a programar. |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | - |

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[server/cron.ts:402](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L402)

***

### monthly \{#monthly\}

▸ **monthly**&lt;`FuncRef`&gt;(`cronIdentifier`, `schedule`, `functionReference`, `...args`): `void`

Programa una mutación o acción para que se ejecute cada mes.

Ten en cuenta que algunos meses tienen menos días que otros; por ejemplo, una función
programada para ejecutarse el día 30 no se ejecutará en febrero.

```js
crons.monthly(
  "Bill customers at ",
  {
    hourUTC: 17, // (9:30am Pacífico/10:30am Pacífico horario de verano)
    minuteUTC: 30,
    day: 1,
  },
  api.billing.billCustomers
)
```

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `FuncRef` | extiende [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | Un nombre único para esta tarea programada. |
| `schedule` | `Monthly` | El día y la hora (UTC) de cada mes en que se ejecutará esta función. |
| `functionReference` | `FuncRef` | Un [FunctionReference](../modules/server.md#functionreference) para la función que se va a programar. |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | Los argumentos de la función. |

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[server/cron.ts:443](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L443)

***

### cron \{#cron\}

▸ **cron**&lt;`FuncRef`&gt;(`cronIdentifier`, `cron`, `functionReference`, `...args`): `void`

Programa una mutación o una acción para que se ejecute de manera periódica.

Al igual que el comando de Unix `cron`, el domingo es 0, el lunes es 1, etc.

```
 ┌─ minuto (0 - 59)
 │ ┌─ hora (0 - 23)
 │ │ ┌─ día del mes (1 - 31)
 │ │ │ ┌─ mes (1 - 12)
 │ │ │ │ ┌─ día de la semana (0 - 6) (domingo a sábado)
"* * * * *"
```

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `FuncRef` | extends [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | Un nombre único para esta tarea programada. |
| `cron` | `string` | Una cadena cron como `"15 7 * * *"` (todos los días a las 7:15 UTC). |
| `functionReference` | `FuncRef` | Un [FunctionReference](../modules/server.md#functionreference) para la función que se va a programar. |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | Los argumentos de la función. |

#### Devuelve \{#returns\}

`void`

#### Definido en \{#defined-in\}

[server/cron.ts:480](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L480)