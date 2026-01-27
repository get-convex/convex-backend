---
title: "Kotlin 与 Convex 类型转换"
sidebar_label: "数据类型"
hidden: false
sidebar_position: 5
description:
  "在 Kotlin 应用程序与 Convex 之间自定义和转换数据类型"
---

## 自定义数据类型 \{#custom-data-types\}

从 Convex 接收值时，你并不局限于使用基本类型。你可以创建自定义的 `@Serializable` 类，这些类会从响应数据中自动解码。

考虑一个 Convex 查询函数，它返回类似下面这个 JavaScript 对象的结果：

```jsx
{
	name: "Guardians",
	uniformColors: ["blue", "white", "red"],
	wins: 80n,
	losses: 60n
}
```

可以用 Kotlin 表示为：

```kotlin
@Serializable
data class BaseballTeam(
    val name: String,
    val uniformColors: List<String>,
    val wins: @ConvexNum Int,
    val losses: @ConvexNum Int)
```

然后你可以在调用 `subscribe` 时将其作为类型参数传入：

```kotlin
convex.subscribe<Team>("mlb:first_place_team", args = mapOf("division" to "AL Central"))
```

远程函数返回的数据会被反序列化为你的自定义类。

## 数值类型 \{#numerical-types\}

你的 Convex 后端代码是用 JavaScript 编写的，它有两种相对常见的数值类型：`number` 和 `BigInt`。

当某个值被赋予数字字面量（不论是 `42` 还是 `3.14`）时，会使用 `number` 类型。`BigInt` 则可以通过在数字后添加一个结尾的 `n` 来使用，比如 `42n`。尽管存在这两种类型，在 JavaScript 中非常常见的做法是用 `number` 来保存整数或浮点数值。

因此，Convex 会格外小心地对值进行编码，以避免精度丢失。由于从技术上讲，`number` 类型是 IEEE 754 浮点值，因此任何时候你从 Convex 得到一个普通的 `number`，它在 Kotlin 中都会被表示为浮点数。你可以根据需求选择使用 `Double` 或 `Float`，但要注意 `Float` 可能会相较原始值丢失精度。

这也意味着 Kotlin 的 `Long` 类型（64 位）无法安全地存储在 `number` 中（只有 53 位可用于编码整数），因此需要使用 `BigInt`。

这一大段铺垫只是为了说明：为了在 Convex 的响应中表示数值，你需要向 Kotlin 提示，这些数值应该使用自定义解码。

你可以通过三种方式来实现这一点。根据你的项目选择最合适的一种即可。

1. 在普通 Kotlin 类型（`Int`、`Long`、`Float`、`Double`）上添加
   `@ConvexNum` 注解
2. 对这些类型使用提供的类型别名（`Int32`、`Int64`、`Float32`、
   `Float64`）
3. 在任何定义了 `@Serializable` 类的文件顶部添加一个特殊注解，
   然后直接使用不带任何注解的普通类型

   ```kotlin
   @file:UseSerializers(
       Int64ToIntDecoder::class,
       Int64ToLongDecoder::class,
       Float64ToFloatDecoder::class,
       Float64ToDoubleDecoder::class
   )

   package com.example.convexapp

   import kotlinx.serialization.UseSerializers

   // @Serializable classes and things.
   ```

在这个示例中，JavaScript 的 `BigInt` 类型是通过在 `wins` 和 `losses` 的值后面添加结尾的 `n` 来使用的，这使得 Kotlin 代码可以使用 `Int`。如果代码改用常规 JavaScript `number` 类型，在 Kotlin 端这些值会被接收到为浮点数，并且反序列化会失败。

如果你遇到那种虽然使用的是 `number` 类型，但按照约定只包含整数值的情况，你可以在你的 `@Serializable` 类中处理这种情况。

```kotlin
@Serializable
data class BaseballTeam(
    val name: String,
    val uniformColors: List<String>,
    @SerialName("wins") private val internalWins: Double,
    @SerialName("losses") private val internalLosses: Double) {

    // 将 JavaScript 的 number 值暴露为 Int。
    val wins get() = internalWins.toInt()
    val losses get() = internalLosses.toInt()
}
```

这个模式是将 `Double` 值私有地存储起来，并使用与后端返回的值不同的字段名。
然后添加访问器来提供 `Int` 值。

## 字段名转换 \{#field-name-conversion\}

上面已经使用过这种模式，但它本身也值得单独说明。有时后端会生成一个
键是 Kotlin 关键字（如 `{fun: true}`），或者键不符合 Kotlin 命名约定（例如以下划线开头）的值。
你可以使用 `@SerialName` 来处理这些情况。

例如，下面展示了如何从后端响应中接收 Convex
[文档 ID](https://docs.convex.dev/database/document-ids)，并将其转换为不会触发 Kotlin lint 警告的字段名：

```kotlin
@Serializable
data class ConvexDocument(@SerialName("_id") val id: String)
```
