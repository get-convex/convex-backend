---
id: "server.Crons"
title: "类：Crons"
custom_edit_url: null
---

[server](../modules/server.md).Crons

用于调度 cron 作业的类。

要了解更多信息，请参阅文档：https://docs.convex.dev/scheduling/cron-jobs

## 构造函数 \{#constructors\}

### 构造函数 \{#constructor\}

• **new Crons**()

#### 定义于 \{#defined-in\}

[server/cron.ts:246](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L246)

## 属性 \{#properties\}

### crons \{#crons\}

• **crons**: `Record`&lt;`string`, [`CronJob`](../interfaces/server.CronJob.md)&gt;

#### 定义于 \{#defined-in\}

[server/cron.ts:244](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L244)

***

### isCrons \{#iscrons\}

• **isCrons**: `true`

#### 定义于 \{#defined-in\}

[server/cron.ts:245](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L245)

## 方法 \{#methods\}

### interval \{#interval\}

▸ **interval**&lt;`FuncRef`&gt;(`cronIdentifier`, `schedule`, `functionReference`, `...args`): `void`

安排一个变更或操作按指定时间间隔运行。

```js
crons.interval("清除在线状态数据", {seconds: 30}, api.presence.clear);
```

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `FuncRef` | 必须扩展自 [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### 参数 \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | - |
| `schedule` | `Interval` | 此定时任务每次运行之间的时间间隔。 |
| `functionReference` | `FuncRef` | 要安排执行的函数的 [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | 传递给该函数的参数。 |

#### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[server/cron.ts:283](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L283)

***

### hourly \{#hourly\}

▸ **hourly**&lt;`FuncRef`&gt;(`cronIdentifier`, `schedule`, `functionReference`, `...args`): `void`

将某个变更或操作设置为按小时运行。

```js
crons.hourly(
  "Reset high scores",
  {
    minuteUTC: 30,
  },
  api.scores.reset
)
```

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `FuncRef` | 受限为 [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) 的子类型 |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | 此定时任务的唯一名称。 |
| `schedule` | `Hourly` | 每天在 UTC 的哪个时间点运行此函数。 |
| `functionReference` | `FuncRef` | 要调度的函数的 [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | 传递给该函数的参数。 |

#### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[server/cron.ts:331](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L331)

***

### daily \{#daily\}

▸ **daily**&lt;`FuncRef`&gt;(`cronIdentifier`, `schedule`, `functionReference`, `...args`): `void`

将某个变更或操作安排为每天运行。

```js
crons.daily(
  "Reset high scores",
  {
    hourUTC: 17, // (太平洋标准时间上午 9:30 / 太平洋夏令时间上午 10:30)
    minuteUTC: 30,
  },
  api.scores.reset
)
```

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `FuncRef` | 继承自 [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | 此定时任务的唯一名称。 |
| `schedule` | `Daily` | 每天在何时（UTC）运行此函数。 |
| `functionReference` | `FuncRef` | 要调度的函数的 [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | 传递给该函数的参数。 |

#### 返回 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[server/cron.ts:366](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L366)

***

### weekly \{#weekly\}

▸ **weekly**&lt;`FuncRef`&gt;(`cronIdentifier`, `schedule`, `functionReference`, `...args`): `void`

按周定期调度运行变更或操作。

```js
crons.weekly(
  "Weekly re-engagement email",
  {
    dayOfWeek: "Tuesday",
    hourUTC: 17, // (太平洋时间上午 9:30/太平洋夏令时间上午 10:30)
    minuteUTC: 30,
  },
  api.emails.send
)
```

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `FuncRef` | 扩展自 [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | 此定时任务的唯一名称。 |
| `schedule` | `Weekly` | 每周在（UTC）哪一天、什么时间运行此函数。 |
| `functionReference` | `FuncRef` | 要调度的函数的 [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | - |

#### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[server/cron.ts:402](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L402)

***

### monthly \{#monthly\}

▸ **monthly**&lt;`FuncRef`&gt;(`cronIdentifier`, `schedule`, `functionReference`, `...args`): `void`

按月调度某个变更或操作运行。

请注意，有些月份的天数比其他月份少，因此，例如计划在每月 30 日运行的函数在二月将不会执行。

```js
crons.monthly(
  "Bill customers at ",
  {
    hourUTC: 17, // (太平洋标准时间上午9:30/太平洋夏令时间上午10:30)
    minuteUTC: 30,
    day: 1,
  },
  api.billing.billCustomers
)
```

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `FuncRef` | 扩展自 [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### 参数 \{#parameters\}

| 名称 | 类型 | 说明 |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | 此计划任务的唯一名称。 |
| `schedule` | `Monthly` | 每个月在（UTC）哪个日期和时间运行此函数。 |
| `functionReference` | `FuncRef` | 要调度的函数的 [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | 传递给该函数的参数。 |

#### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[server/cron.ts:443](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L443)

***

### cron \{#cron\}

▸ **cron**&lt;`FuncRef`&gt;(`cronIdentifier`, `cron`, `functionReference`, `...args`): `void`

安排一个变更或操作定期运行。

与 unix 命令 `cron` 类似，星期日为 0，星期一为 1，依此类推。

```
 ┌─ minute (0 - 59)
 │ ┌─ hour (0 - 23)
 │ │ ┌─ day of the month (1 - 31)
 │ │ │ ┌─ month (1 - 12)
 │ │ │ │ ┌─ day of the week (0 - 6) (Sunday to Saturday)
"* * * * *"
```

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `FuncRef` | extends [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | 此定时任务的唯一名称。 |
| `cron` | `string` | Cron 表达式字符串，例如 `"15 7 * * *"`（每天 UTC 时间 7:15）。 |
| `functionReference` | `FuncRef` | 要调度的函数的 [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | 传递给该函数的参数。 |

#### 返回值 \{#returns\}

`void`

#### 定义于 \{#defined-in\}

[server/cron.ts:480](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L480)