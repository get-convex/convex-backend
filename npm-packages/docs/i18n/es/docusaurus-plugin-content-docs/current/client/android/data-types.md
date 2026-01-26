---
title: "Conversión de tipos entre Kotlin y Convex"
sidebar_label: "Tipos de datos"
hidden: false
sidebar_position: 5
description:
  "Personalizar y convertir tipos entre la aplicación Kotlin y Convex"
---

## Tipos de datos personalizados \{#custom-data-types\}

Al recibir valores de Convex, no estás limitado a valores primitivos. Puedes
crear clases `@Serializable` personalizadas que se decodificarán automáticamente
a partir de los datos de la respuesta.

Considera una función de consulta de Convex que devuelve resultados como este
objeto de JavaScript:

```jsx
{
	name: "Guardians",
	uniformColors: ["blue", "white", "red"],
	wins: 80n,
	losses: 60n
}
```

Esto se puede representar en Kotlin mediante:

```kotlin
@Serializable
data class BaseballTeam(
    val name: String,
    val uniformColors: List<String>,
    val wins: @ConvexNum Int,
    val losses: @ConvexNum Int)
```

Luego puedes pasarlo como argumento de tipo en tu llamada a `subscribe`:

```kotlin
convex.subscribe<Team>("mlb:first_place_team", args = mapOf("division" to "AL Central"))
```

Los datos devueltos por la función remota se deserializarán en tu clase personalizada.

## Tipos numéricos \{#numerical-types\}

El código de tu backend de Convex está escrito en JavaScript, que tiene dos tipos
relativamente comunes para datos numéricos: `number` y `BigInt`.

`number` se usa siempre que a una variable se le asigna un valor numérico literal,
ya sea `42` o `3.14`. `BigInt` se puede usar añadiendo una `n` al final, como
`42n`. A pesar de estos dos tipos, es muy común usar `number` para almacenar
tanto valores enteros como de punto flotante en JavaScript.

Por este motivo, Convex tiene especial cuidado al codificar valores para que no
pierdan precisión. Dado que técnicamente el tipo `number` es un valor de punto
flotante IEEE 754, cada vez que obtengas un `number` simple de Convex se
representará como punto flotante en Kotlin. Puedes elegir usar `Double` o
`Float`, según tus necesidades, pero ten en cuenta que `Float` podría perder
precisión respecto al original.

Esto también significa que el tipo `Long` de Kotlin (64 bits) no puede
almacenarse de forma segura en un `number` (solo hay 53 bits disponibles para
codificar enteros) y requiere un `BigInt`.

Todo esto es una larga introducción para explicar que, para representar valores
numéricos en respuestas de Convex, necesitas indicar a Kotlin que debe usar
decodificación personalizada.

Puedes hacer esto de tres maneras. Usa la que resulte más útil para tu
proyecto.

1. Anota el tipo básico de Kotlin (`Int`, `Long`, `Float`, `Double`) con
   `@ConvexNum`
2. Usa un alias de tipo proporcionado para esos tipos (`Int32`, `Int64`,
   `Float32`, `Float64`)
3. Incluye una anotación especial al principio de cualquier archivo que defina
   clases `@Serializable` y simplemente usa los tipos básicos sin anotación

   ```kotlin
   @file:UseSerializers(
       Int64ToIntDecoder::class,
       Int64ToLongDecoder::class,
       Float64ToFloatDecoder::class,
       Float64ToDoubleDecoder::class
   )

   package com.example.convexapp

   import kotlinx.serialization.UseSerializers

   // Clases @Serializable y demás.
   ```

En el ejemplo, el tipo `BigInt` de JavaScript se usa añadiendo una `n` al final
de los valores `wins` y `losses`, lo que permite que el código de Kotlin use
`Int`. Si en su lugar el código usara el tipo `number` normal de JavaScript, en
el lado de Kotlin estos se recibirían como valores de punto flotante y la
deserialización fallaría.

Si tienes una situación como esa en la que se usa `number` pero por convención
solo contiene valores enteros, puedes gestionarlo en tu clase `@Serializable`.

```kotlin
@Serializable
data class BaseballTeam(
    val name: String,
    val uniformColors: List<String>,
    @SerialName("wins") private val internalWins: Double,
    @SerialName("losses") private val internalLosses: Double) {

    // Expone los valores number de JavaScript como Ints.
    val wins get() = internalWins.toInt()
    val losses get() = internalLosses.toInt()
}
```

El patrón consiste en almacenar los valores `Double` de forma privada y con nombres diferentes
de los valores que vienen del backend. Luego añade métodos de acceso para proporcionar los valores `Int`.

## Conversión de nombres de campos \{#field-name-conversion\}

Este patrón se usó anteriormente, pero vale la pena describirlo por sí mismo. A veces, se
generará un valor en el backend con una clave que coincide con una palabra clave de Kotlin
(`{fun: true}`) o que no cumple las convenciones de nomenclatura de Kotlin (por ejemplo, empieza
con un guion bajo). Puedes usar `@SerialName` para manejar esos casos.

Por ejemplo, así es como puedes obtener el
[ID de documento](https://docs.convex.dev/database/document-ids) de Convex desde una respuesta
del backend y convertirlo en un nombre de campo que no genere advertencias de lint en Kotlin:

```kotlin
@Serializable
data class ConvexDocument(@SerialName("_id") val id: String)
```
