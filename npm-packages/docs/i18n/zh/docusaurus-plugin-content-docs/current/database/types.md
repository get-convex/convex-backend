---
title: "数据类型"
sidebar_position: 40
description: "Convex 文档支持的数据类型"
---

import ConvexValues from "@site/i18n/zh/docusaurus-plugin-content-docs/current/\_convexValues.mdx";

所有 Convex 文档都定义为 JavaScript 对象。这些对象的字段值可以是下列任意类型之一。

你可以通过在数据表中[定义模式](/database/schemas.mdx)来规定文档的结构。

## Convex 值类型 \{#convex-values\}

<ConvexValues />

## 系统字段 \{#system-fields\}

Convex 中的每个文档都有两个自动生成的系统字段：

* `_id`：该文档的[文档 ID](/database/document-ids.mdx)。
* `_creationTime`：此文档的创建时间，以自 Unix 纪元以来经过的毫秒数表示。

## 限制 \{#limits\}

Convex 值的总大小必须小于 1MB。当前这是一个大致的限制，但如果你遇到这些限制并希望有一种更精确的方法来计算文档的大小，
[请联系我们](https://convex.dev/community)。文档可以包含嵌套的值，即包含其他 Convex 类型的对象或数组。Convex 类型最多可以有 16 层嵌套，并且嵌套值树的累计大小必须小于 1MB。

表名可以包含字母和数字字符（&quot;a&quot; 到 &quot;z&quot;、&quot;A&quot; 到 &quot;Z&quot; 和 &quot;0&quot; 到 &quot;9&quot;）以及下划线（&quot;&#95;&quot;），但不能以下划线开头。

关于其他限制的信息，请参见[此处](/production/state/limits.mdx)。

如果这些限制无法满足你的需求，
[请告诉我们](https://convex.dev/community)！

## 使用 `undefined` \{#working-with-undefined\}

TypeScript 中的值 `undefined` 不是合法的 Convex 值，因此不能在 Convex 函数的参数或返回值中使用，也不能用于存储到文档中。

1. 含有 `undefined` 值的对象/record 与缺少该字段时是相同的：`{a: undefined}` 在传递给函数或存入数据库时会被转换为 `{}`。你可以将 Convex 函数调用和 Convex 数据库理解为用 `JSON.stringify` 来序列化数据，它同样会移除 `undefined` 值。
2. 对象字段的校验器可以使用 `v.optional(...)` 来表示该字段可能不存在。
   * 如果对象的字段 &quot;a&quot; 缺失，即 `const obj = {};`，那么 `obj.a === undefined`。这是 TypeScript/JavaScript 的特性，与 Convex 无关。
3. 你可以在过滤器和索引查询中使用 `undefined`，它会匹配那些没有该字段的文档。即
   `.withIndex("by_a", q=>q.eq("a", undefined))` 会匹配文档 `{}` 和
   `{b: 1}`，但不会匹配 `{a: 1}` 或 `{a: null, b: 1}`。
   * 在 Convex 的排序方案中，`undefined < null < 所有其他值`，因此你可以通过 `q.gte("a", null as any)` 或
     `q.gt("a", undefined)` 来匹配 *具有* 该字段的文档。
4. 只有一种情况 `{a: undefined}` 与 `{}` 不同：当传给 `ctx.db.patch` 时。传入 `{a: undefined}` 会从文档中移除字段 &quot;a&quot;，而传入 `{}` 则不会改变字段 &quot;a&quot;。参见
   [更新已有文档](/database/writing-data.mdx#updating-existing-documents)。
5. 由于 `undefined` 会从函数参数中被剥离，但在 `ctx.db.patch` 中具有意义，因此从客户端传递给 patch 的参数时有一些技巧。
   * 如果客户端传递的 `args={}`（或等价的 `args={a: undefined}`）应该保持字段 &quot;a&quot; 不变，使用
     `ctx.db.patch(id, args)`。
   * 如果客户端传递的 `args={}` 应该移除字段 &quot;a&quot;，使用
     `ctx.db.patch(id, {a: undefined, ...args})`。
   * 如果客户端传递的 `args={}` 应该保持字段 &quot;a&quot; 不变，而 `args={a: null}` 应该移除它，你可以这样做：
     ```ts
     if (args.a === null) {
       args.a = undefined;
     }
     await ctx.db.patch(tableName, id, args);
     ```
6. 返回普通 `undefined`/`void` 的函数会被视为它们返回了 `null`。
7. 包含 `undefined` 值的数组（如 `[undefined]`）在作为 Convex 值使用时会抛出错误。

如果你希望避免 `undefined` 的这些特殊行为，可以改用 `null`，它 *是* 一个合法的 Convex 值。

## 处理日期和时间 \{#working-with-dates-and-times\}

Convex 没有用于处理日期和时间的特殊数据类型。你如何存储日期取决于应用的需求：

1. 如果你只关心某个时间点，你可以存储一个
   [UTC 时间戳](https://en.wikipedia.org/wiki/Unix_time)。我们推荐参考 `_creationTime` 字段的用法，它将时间戳以毫秒为单位存储为一个
   `number`。在你的函数和客户端中，可以通过将时间戳传入 JavaScript `Date` 构造函数来创建一个 `Date` 实例：
   `new Date(timeInMsSinceEpoch)`。然后你可以按所需的时区（例如用户机器配置的时区）来输出/显示日期和时间。
   * 要在函数中获取当前 UTC 时间戳并将其存入数据库，使用 `Date.now()`
2. 如果你关心的是日历日期或具体的时间点，例如在实现预订类应用时，你应该将实际的日期和/或时间存为字符串。如果你的应用支持多个时区，你还应该存储时区。[ISO8601](https://en.wikipedia.org/wiki/ISO_8601) 是一种常见的格式，用于在一个字符串中同时存储日期和时间，例如
   `"2024-03-21T14:37:15Z"`。如果用户可以选择特定的时区，你通常应该将其存储在单独的 `string` 字段中，通常使用
   [IANA 时区名称](https://en.wikipedia.org/wiki/Tz_database#Names_of_time_zones)
   （当然你也可以使用像 `"|"` 这样的特殊字符把这两个字段拼接起来）。

要进行更复杂的日期和时间格式化和操作，请使用流行的 JavaScript 库之一：[date-fns](https://date-fns.org/)、[Day.js](https://day.js.org/)、[Luxon](https://moment.github.io/luxon/) 或 [Moment.js](https://momentjs.com/)。