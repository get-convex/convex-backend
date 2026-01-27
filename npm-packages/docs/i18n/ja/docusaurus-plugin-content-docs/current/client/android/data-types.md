---
title: "Kotlin と Convex の型変換"
sidebar_label: "データ型"
hidden: false
sidebar_position: 5
description:
  "Kotlin アプリと Convex の間での型のカスタマイズと変換"
---

## カスタムデータ型 \{#custom-data-types\}

Convex から値を受け取るとき、プリミティブな値だけに限定されるわけではありません。
レスポンスデータから自動的にデコードされるカスタムの `@Serializable` クラスを作成できます。

次のような JavaScript オブジェクトを返す Convex のクエリ関数があるとします。

```jsx
{
	name: "Guardians",
	uniformColors: ["blue", "white", "red"],
	wins: 80n,
	losses: 60n
}
```

これは Kotlin では次のように記述できます:

```kotlin
@Serializable
data class BaseballTeam(
    val name: String,
    val uniformColors: List<String>,
    val wins: @ConvexNum Int,
    val losses: @ConvexNum Int)
```

その後は、その型を `subscribe` 呼び出しの型引数として渡せます。

```kotlin
convex.subscribe<Team>("mlb:first_place_team", args = mapOf("division" to "AL Central"))
```

リモート関数から取得したデータは、独自のクラスにデシリアライズされます。

## 数値型 \{#numerical-types\}

Convex のバックエンドコードは JavaScript で書かれており、数値データに対してよく使われる型は `number` と `BigInt` の 2 つです。

`number` は `42` や `3.14` のように、リテラルな数値が代入されたときに使われます。`BigInt` は `42n` のように末尾に `n` を付けることで使えます。2 種類の型がありますが、JavaScript では整数でも浮動小数点数でも、`number` を使って保持するのが非常に一般的です。

このため Convex は、値が精度を失わないように特別な注意を払ってエンコードします。技術的には `number` 型は IEEE 754 浮動小数点数の値なので、Convex からプレーンな `number` を取得した場合、それは Kotlin では浮動小数点として表現されます。用途に応じて `Double` か `Float` を選べますが、`Float` は元の値から精度を失う可能性があることに注意してください。

これはまた、Kotlin の `Long` 型（64 ビット）は `number` に安全に格納できない（整数をエンコードするのに使えるのは 53 ビットだけ）ため、`BigInt` が必要になることも意味します。

前置きが長くなりましたが、Convex からのレスポンス内の数値を表現するためには、Kotlin に対してカスタムデコード処理を使うべきだと示す必要があります。

これは 3 つの方法で行えます。プロジェクトにとって最も便利な方法を使ってください。

1. プレーンな Kotlin 型（`Int`, `Long`, `Float`, `Double`）に
   `@ConvexNum` を付与する
2. それらの型向けに提供されている型エイリアス（`Int32`, `Int64`, `Float32`,
   `Float64`）を使う
3. `@Serializable` クラスを定義しているファイルの先頭に特別なアノテーションを付け、クラス内ではプレーンな型をアノテーションなしで使う

   ```kotlin
   @file:UseSerializers(
       Int64ToIntDecoder::class,
       Int64ToLongDecoder::class,
       Float64ToFloatDecoder::class,
       Float64ToDoubleDecoder::class
   )

   package com.example.convexapp

   import kotlinx.serialization.UseSerializers

   // @Serializable クラスなど。
   ```

この例では、JavaScript の `BigInt` 型が `wins` と `losses` の値に末尾の `n` を付けることで使われており、そのおかげで Kotlin コードは `Int` を使えます。代わりに通常の JavaScript の `number` 型を使った場合、Kotlin 側ではそれらは浮動小数点の値として受け取られ、デシリアライズは失敗します。

`number` が使われているものの、慣例として整数のみを保持するようなケースがある場合は、その処理を `@Serializable` クラス内で行うことができます。

```kotlin
@Serializable
data class BaseballTeam(
    val name: String,
    val uniformColors: List<String>,
    @SerialName("wins") private val internalWins: Double,
    @SerialName("losses") private val internalLosses: Double) {

    // JavaScriptのnumber値をIntとして公開する。
    val wins get() = internalWins.toInt()
    val losses get() = internalLosses.toInt()
}
```

このパターンでは、`Double` の値をバックエンドからの値とは異なる名前で、かつプライベートに保持します。
そのうえで、`Int` の値を提供するアクセサを追加します。

## フィールド名の変換 \{#field-name-conversion\}

このパターンはすでに上で使いましたが、単独でも説明しておきます。ときどき、
バックエンド側で生成された値のキーが Kotlin のキーワード（`{fun: true}`）と一致したり、
Kotlin の命名規則に従っていない（例: 先頭がアンダースコア）場合があります。そうしたケースには
`@SerialName` を使うことができます。

たとえば、バックエンドのレスポンスから Convex の
[document ID](https://docs.convex.dev/database/document-ids) を取得し、
Kotlin の Lint 警告を引き起こさないフィールド名に変換する方法は次のとおりです:

```kotlin
@Serializable
data class ConvexDocument(@SerialName("_id") val id: String)
```
