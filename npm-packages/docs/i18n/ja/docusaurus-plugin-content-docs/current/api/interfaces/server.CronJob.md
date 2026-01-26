---
id: "server.CronJob"
title: "インターフェース: CronJob"
custom_edit_url: null
---

[server](../modules/server.md).CronJob

Convex のミューテーションまたはアクションを実行するためのスケジュールです。
interval を使って Convex 関数を定期的に実行するようにスケジュールし、
そのスケジュールをエクスポートできます。

## プロパティ \{#properties\}

### name \{#name\}

• **name**: `string`

#### 定義元 \{#defined-in\}

[server/cron.ts:153](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L153)

***

### args \{#args\}

• **args**: [`JSONValue`](../modules/values.md#jsonvalue)

#### 定義場所 \{#defined-in\}

[server/cron.ts:154](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L154)

***

### schedule \{#schedule\}

• **schedule**: `Schedule`

#### 定義元 \{#defined-in\}

[server/cron.ts:155](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L155)