---
id: "server.Crons"
title: "クラス: Crons"
custom_edit_url: null
---

[server](../modules/server.md).Crons

cron ジョブのスケジューリング用のクラスです。

詳しくは https://docs.convex.dev/scheduling/cron-jobs のドキュメントを参照してください。

## コンストラクタ \{#constructors\}

### コンストラクタ \{#constructor\}

• **new Crons**()

#### 定義元 \{#defined-in\}

[server/cron.ts:246](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L246)

## プロパティ \{#properties\}

### crons \{#crons\}

• **crons**: `Record`&lt;`string`, [`CronJob`](../interfaces/server.CronJob.md)&gt;

#### 定義場所 \{#defined-in\}

[server/cron.ts:244](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L244)

***

### isCrons \{#iscrons\}

• **isCrons**: `true`

#### 定義場所 \{#defined-in\}

[server/cron.ts:245](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L245)

## メソッド \{#methods\}

### interval \{#interval\}

▸ **interval**&lt;`FuncRef`&gt;(`cronIdentifier`, `schedule`, `functionReference`, `...args`): `void`

ミューテーションまたはアクションを一定間隔で実行するようにスケジュールします。

```js
crons.interval("Clear presence data", {seconds: 30}, api.presence.clear);
```

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `FuncRef` | extends [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | - |
| `schedule` | `Interval` | このスケジュールされたジョブの実行間隔。 |
| `functionReference` | `FuncRef` | スケジュールする関数の [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | 関数に渡す引数。 |

#### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[server/cron.ts:283](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L283)

***

### hourly \{#hourly\}

▸ **hourly**&lt;`FuncRef`&gt;(`cronIdentifier`, `schedule`, `functionReference`, `...args`): `void`

ミューテーションやアクションを、1時間ごとに実行されるようにスケジュールします。

```js
crons.hourly(
  "Reset high scores",
  {
    minuteUTC: 30,
  },
  api.scores.reset
)
```

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `FuncRef` | extends [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | このスケジュールされたジョブを識別する一意の名前。 |
| `schedule` | `Hourly` | この関数を毎日実行する時刻 (UTC)。 |
| `functionReference` | `FuncRef` | スケジュールする関数の [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | 関数に渡す引数。 |

#### 戻り値 \{#returns\}

`void`

#### 定義元 \{#defined-in\}

[server/cron.ts:331](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L331)

***

### daily \{#daily\}

▸ **daily**&lt;`FuncRef`&gt;(`cronIdentifier`, `schedule`, `functionReference`, `...args`): `void`

ミューテーションまたはアクションを毎日自動的に実行するようスケジュールします。

```js
crons.daily(
  "Reset high scores",
  {
    hourUTC: 17, // (太平洋標準時午前9:30/太平洋夏時間午前10:30)
    minuteUTC: 30,
  },
  api.scores.reset
)
```

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `FuncRef` | extends [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | この定期実行ジョブの一意な名前。 |
| `schedule` | `Daily` | この関数を毎日 (UTC) の何時に実行するか。 |
| `functionReference` | `FuncRef` | スケジュールする関数の [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | 関数に渡す引数。 |

#### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[server/cron.ts:366](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L366)

***

### weekly \{#weekly\}

▸ **weekly**&lt;`FuncRef`&gt;(`cronIdentifier`, `schedule`, `functionReference`, `...args`): `void`

ミューテーションまたはアクションが毎週実行されるようにスケジュールします。

```js
crons.weekly(
  "Weekly re-engagement email",
  {
    dayOfWeek: "Tuesday",
    hourUTC: 17, // (太平洋標準時午前9:30/太平洋夏時間午前10:30)
    minuteUTC: 30,
  },
  api.emails.send
)
```

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `FuncRef` | [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) を拡張する型 |

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | このスケジュールされたジョブの一意の名前。 |
| `schedule` | `Weekly` | この関数を毎週実行する曜日と時刻 (UTC)。 |
| `functionReference` | `FuncRef` | スケジュールする関数の [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | - |

#### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[server/cron.ts:402](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L402)

***

### monthly \{#monthly\}

▸ **monthly**&lt;`FuncRef`&gt;(`cronIdentifier`, `schedule`, `functionReference`, `...args`): `void`

ミューテーションまたはアクションが毎月実行されるようにスケジュールします。

月によって日数が異なるため、たとえば30日に実行するようスケジュールされた関数は、2月には実行されない点に注意してください。

```js
crons.monthly(
  "Bill customers at ",
  {
    hourUTC: 17, // (太平洋標準時午前9:30/太平洋夏時間午前10:30)
    minuteUTC: 30,
    day: 1,
  },
  api.billing.billCustomers
)
```

#### 型パラメータ \{#type-parameters\}

| 名称 | 型 |
| :------ | :------ |
| `FuncRef` | extends [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | このスケジュール済みジョブの一意の識別名。 |
| `schedule` | `Monthly` | この関数を毎月実行する UTC の日付と時刻。 |
| `functionReference` | `FuncRef` | スケジュールする関数の [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | 関数に渡す引数。 |

#### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[server/cron.ts:443](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L443)

***

### cron \{#cron\}

▸ **cron**&lt;`FuncRef`&gt;(`cronIdentifier`, `cron`, `functionReference`, `...args`): `void`

ミューテーションまたはアクションを定期的に実行するようスケジューリングします。

Unix コマンドの `cron` と同様に、日曜日を 0、月曜日を 1 とします。

```
 ┌─ minute (0 - 59)
 │ ┌─ hour (0 - 23)
 │ │ ┌─ day of the month (1 - 31)
 │ │ │ ┌─ month (1 - 12)
 │ │ │ │ ┌─ day of the week (0 - 6) (Sunday to Saturday)
"* * * * *"
```

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `FuncRef` | extends [`SchedulableFunctionReference`](../modules/server.md#schedulablefunctionreference) |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `cronIdentifier` | `string` | このジョブをスケジュールするための一意の名前。 |
| `cron` | `string` | `"15 7 * * *"`（毎日 UTC の 7:15）のような Cron 形式の文字列。 |
| `functionReference` | `FuncRef` | スケジュールする関数の [FunctionReference](../modules/server.md#functionreference)。 |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`FuncRef`&gt; | 関数に渡す引数。 |

#### 戻り値 \{#returns\}

`void`

#### 定義場所 \{#defined-in\}

[server/cron.ts:480](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L480)