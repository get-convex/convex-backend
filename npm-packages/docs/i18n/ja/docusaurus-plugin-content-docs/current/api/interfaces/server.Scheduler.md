---
id: "server.Scheduler"
title: "インターフェース: Scheduler"
custom_edit_url: null
---

[server](../modules/server.md).Scheduler

Convex の関数をスケジュールするためのインターフェースです。

ミューテーションかアクションのいずれかをスケジュールできます。ミューテーションは
必ずちょうど 1 回実行されることが保証されています。一時的なエラーが発生した場合は
自動的にリトライされ、正常に完了するか、関数定義における開発者のエラーによって
決定的に失敗するかのいずれかになります。アクションは最大 1 回だけ実行されます。
リトライは行われず、一時的なエラーによって失敗する可能性があります。

これらの関数が Convex クライアントから直接呼び出されないようにするには、
internalMutation または internalAction の使用を検討してください。

## メソッド \{#methods\}

### runAfter \{#runafter\}

▸ **runAfter**&lt;`FuncRef`&gt;(`delayMs`, `functionReference`, `...args`): `Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_scheduled_functions"`&gt;&gt;

指定した遅延時間後に実行されるように関数をスケジュールします。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `FuncRef` | extends [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `delayMs` | `number` | 遅延時間（ミリ秒）。0以上である必要があります。遅延が 0 の場合、スケジュールされた関数は、スケジューリングを行う関数の実行が完了した直後に実行されます。 |
| `functionReference` | `FuncRef` | スケジュール対象の関数を示す [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | スケジュールされた関数を呼び出す際に渡す引数。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_scheduled_functions"`&gt;&gt;

#### 定義元 \{#defined-in\}

[server/scheduler.ts:41](https://github.com/get-convex/convex-js/blob/main/src/server/scheduler.ts#L41)

***

### runAt \{#runat\}

▸ **runAt**&lt;`FuncRef`&gt;(`timestamp`, `functionReference`, `...args`): `Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_scheduled_functions"`&gt;&gt;

指定したタイムスタンプの時刻に実行されるように関数をスケジュールします。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `FuncRef` | extends [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `timestamp` | `number` | `Date` | `Date` オブジェクト、またはタイムスタンプ（エポックからの経過ミリ秒）。タイムスタンプが過去の場合、スケジュールされた関数はスケジューリングを行った関数の実行完了直後にすぐに実行されるようスケジュールされます。タイムスタンプは、5 年以上前や 5 年以上先の時刻を指定することはできません。 |
| `functionReference` | `FuncRef` | スケジュールする関数を指定するための [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | スケジュールされた関数を呼び出す際に渡す引数。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`GenericId`](../modules/values.md#genericid)&lt;`"_scheduled_functions"`&gt;&gt;

#### 定義元 \{#defined-in\}

[server/scheduler.ts:58](https://github.com/get-convex/convex-js/blob/main/src/server/scheduler.ts#L58)

***

### cancel \{#cancel\}

▸ **cancel**(`id`): `Promise`&lt;`void`&gt;

まだ開始されていない、事前にスケジュールされた関数をキャンセルします。
スケジュールされた関数がすでに実行中の場合、その関数の実行は継続されますが、
その関数が新たにスケジュールしようとする関数はすべてキャンセルされます。

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `id` | [`GenericId`](../modules/values.md#genericid)&lt;`"_scheduled_functions"`&gt; |

#### 戻り値 \{#returns\}

`Promise`&lt;`void`&gt;

#### 定義場所 \{#defined-in\}

[server/scheduler.ts:71](https://github.com/get-convex/convex-js/blob/main/src/server/scheduler.ts#L71)