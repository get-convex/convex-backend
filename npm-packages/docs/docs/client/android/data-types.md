---
title: "Kotlin and Convex type conversion"
sidebar_label: "Data Types"
hidden: false
sidebar_position: 5
description:
  "Customizing and converting types between the Kotlin app and Convex"
---

## Custom data types

When receiving values from Convex, you aren't limited to primitive values. You
can create custom `@Serializable` classes that will be automatically decoded
from response data.

Consider a Convex query function that returns results like this JavaScript
object:

```jsx
{
	name: "Guardians",
	uniformColors: ["blue", "white", "red"],
	wins: 80n,
	losses: 60n
}
```

That can be represented in Kotlin using:

```kotlin
@Serializable
data class BaseballTeam(
    val name: String,
    val uniformColors: List<String>,
    val wins: @ConvexNum Int,
    val losses: @ConvexNum Int)
```

Then you can pass it as the type argument in your `subscribe` call:

```kotlin
convex.subscribe<Team>("mlb:first_place_team", args = mapOf("division" to "AL Central"))
```

The data from the remote function will be deserialized to your custom class.

## Numerical types

Your Convex backend code is written in JavaScript, which has two relatively
common types for numerical data: `number` and `BigInt`.

`number` is used whenever a value is assigned a literal numeric value, whether
`42` or `3.14`. `BigInt` can be used by adding a trailing `n`, like `42n`.
Despite the two types, is very common to use `number` for holding either integer
or floating point values in JavaScript.

Because of this, Convex takes extra care to encode values so they won't lose
precision. Since technically the `number` type is an IEEE 754 floating point
value, anytime you get a plain `number` from Convex it will be represented as
floating point in Kotlin. You can choose to use `Double` or `Float`, depending
on your needs but be aware that `Float` might lose precision from the original.

It also means that Kotlin's `Long` type (64 bit) can't be safely stored in a
`number` (only 53 bits are available to encode integers) and requires a
`BigInt`.

That's a long lead up to explain that in order to represent numerical values in
responses from Convex, you need to hint to Kotlin that they should use custom
decoding.

You can do this in three ways. Use whichever seems most useful to your project.

1. Annotate the plain Kotlin type (`Int`, `Long`, `Float`, `Double`) with
   `@ConvexNum`
2. Use a provided type alias for those types (`Int32`, `Int64`, `Float32`,
   `Float64`)
3. Include a special annotation at the top of any file that defines
   `@Serializable` classes and just use the plain types with no annotation

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

In the example, JavaScript's `BigInt` type is used by adding a trailing `n` to
the `wins` and `losses` values which lets the Kotlin code use `Int`. If instead
the code used regular JavaScript `number` types, on the Kotlin side those would
be received as floating point values and deserialization would fail.

If you have a situation like that where `number` is used but by convention only
contains integer values, you can handle that in your `@Serializable` class.

```kotlin
@Serializable
data class BaseballTeam(
    val name: String,
    val uniformColors: List<String>,
    @SerialName("wins") private val internalWins: Double,
    @SerialName("losses") private val internalLosses: Double) {

    // Expose the JavaScript number values as Ints.
    val wins get() = internalWins.toInt()
    val losses get() = internalLosses.toInt()
}
```

The pattern is to store the `Double` values privately and with different names
that the value from the backend. Then add accessors to provide the `Int` values.

## Field name conversion

This pattern was used above, but it bears describing on its own. Sometimes a
value will be produced on the backend with a key that matches a Kotlin keyword
(`{fun: true}`) or doesn't conform to Kotlin naming conventions (e.g. starts
with an underscore). You can use `@SerialName` to handle those cases.

For example, here's how you can ingest the Convex
[document ID](https://docs.convex.dev/database/document-ids) from a backend
response and convert it to a field name that won't trigger Kotlin lint warnings:

```kotlin
@Serializable
data class ConvexDocument(@SerialName("_id") val id: String)
```
