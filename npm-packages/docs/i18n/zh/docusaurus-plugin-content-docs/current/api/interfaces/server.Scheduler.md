---
id: "server.Scheduler"
title: "接口：Scheduler"
custom_edit_url: null
---

[server](../modules/server.md).Scheduler

用于调度 Convex 函数的接口。

你可以调度变更或操作函数。变更被保证**恰好执行一次**——在出现瞬时错误时会自动重试，并要么成功执行，要么因为开发者在定义函数时的错误而以确定性的方式失败。操作函数**至多执行一次**——它们不会被重试，并且可能因为瞬时错误而失败。

建议使用 `internalMutation` 或 `internalAction` 来确保这些函数不能从 Convex 客户端被直接调用。

## 方法 \{#methods\}

### runAfter \{#runafter\}

▸ **runAfter**&lt;`FuncRef`&gt;(`delayMs`, `functionReference`, `...args`): `Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_scheduled_functions"`&gt;&gt;

安排一个函数在指定延迟后执行。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `FuncRef` | 扩展自 [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `delayMs` | `number` | 以毫秒为单位的延迟。必须为非负数。如果延迟为零，计划执行的函数将在当前调度函数完成后立即执行。 |
| `functionReference` | `FuncRef` | 要调度的函数的 [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | 调用已调度函数时传入的参数。 |

#### 返回 \{#returns\}

`Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_scheduled_functions"`&gt;&gt;

#### 定义于 \{#defined-in\}

[server/scheduler.ts:41](https://github.com/get-convex/convex-js/blob/main/src/server/scheduler.ts#L41)

***

### runAt \{#runat\}

▸ **runAt**&lt;`FuncRef`&gt;(`timestamp`, `functionReference`, `...args`): `Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_scheduled_functions"`&gt;&gt;

安排在给定的时间戳执行一个函数。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `FuncRef` | 扩展自 [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### 参数 \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `timestamp` | `number` | `Date` | 一个 `Date` 或时间戳（自 Unix 纪元以来的毫秒数）。如果该时间戳早于当前时间，被调度的函数会在本次调度调用完成后立即执行。该时间戳不能早于当前时间五年以上，也不能晚于当前时间五年以上。 |
| `functionReference` | `FuncRef` | 要调度的函数的 [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | 调用被调度函数时要传入的参数。 |

#### 返回值 \{#returns\}

`Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_scheduled_functions"`&gt;&gt;

#### 定义于 \{#defined-in\}

[server/scheduler.ts:58](https://github.com/get-convex/convex-js/blob/main/src/server/scheduler.ts#L58)

***

### cancel \{#cancel\}

▸ **cancel**(`id`): `Promise`&lt;`void`&gt;

取消一个已调度但尚未开始执行的函数。如果该调度函数已经在执行中，它会继续运行，但该函数尝试调度的任何新函数都会被取消。

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`"_scheduled_functions"`&gt; |

#### 返回值 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定义于 \{#defined-in\}

[server/scheduler.ts:71](https://github.com/get-convex/convex-js/blob/main/src/server/scheduler.ts#L71)