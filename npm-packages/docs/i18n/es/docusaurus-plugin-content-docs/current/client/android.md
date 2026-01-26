---
title: "Android Kotlin"
sidebar_label: "Android Kotlin"
sidebar_position: 600
description:
  "Biblioteca cliente de Android Kotlin para aplicaciones móviles que usan Convex"
---

La biblioteca cliente de Convex para Android permite que tu aplicación de Android interactúe con tu backend de Convex. Permite que tu código de frontend:

1. Llame
   a tus [consultas](/functions/query-functions.mdx), [mutaciones](/functions/mutation-functions.mdx) y [acciones](/functions/actions.mdx)
2. Autentique usuarios con [Auth0](/auth/auth0.mdx)

La biblioteca es de código abierto y
[está disponible en GitHub](https://github.com/get-convex/convex-mobile/tree/main/android).

Sigue la [guía de inicio rápido de Android](/quickstart/android.mdx) para comenzar.

## Instalación \{#installation\}

Realiza los siguientes cambios en el archivo `build.gradle[.kts]` de tu aplicación.

```kotlin
plugins {
    // ... existing plugins
    kotlin("plugin.serialization") version "1.9.0"
}

dependencies {
    // ... dependencias existentes
    implementation("dev.convex:android-convexmobile:0.4.1@aar") {
        isTransitive = true
    }
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.3")
}
```

Después de eso, sincroniza Gradle para aplicar esos cambios. Tu aplicación ahora tendrá acceso
a la biblioteca de Convex para Android, así como a la serialización JSON de Kotlin,
que se utiliza para comunicarse entre tu código y el backend de Convex.

## Conectarse a un backend \{#connecting-to-a-backend\}

El `ConvexClient` se utiliza para establecer y mantener una conexión entre tu
aplicación y el backend de Convex. Primero debes crear una instancia del
cliente proporcionándole la URL de implementación de tu backend:

```kotlin
package com.example.convexapp

import dev.convex.android.ConvexClient

val convex = ConvexClient("https://<tu dominio aquí>.convex.cloud")
```

Debes crear y usar una sola instancia de `ConvexClient` durante todo el ciclo de vida
del proceso de tu aplicación. Puede resultar conveniente crear en Android una
subclase personalizada de
[`Application`](https://developer.android.com/reference/android/app/Application)
e inicializarla allí:

```kotlin
package com.example.convexapp

import android.app.Application
import dev.convex.android.ConvexClient

class MyApplication : Application() {
    lateinit var convex: ConvexClient

    override fun onCreate() {
        super.onCreate()
        convex = ConvexClient("https://<your domain here>.convex.cloud")
    }
}
```

Una vez que hayas hecho eso, puedes acceder al cliente desde una función
`@Composable` en Jetpack Compose como esta:

```kotlin
val convex = (application as MyApplication).convex
```

## Obtención de datos \{#fetching-data\}

Convex para Android te da acceso al
[reactor](https://docs.convex.dev/tutorial/reactor) de Convex, que permite
*suscripciones* en tiempo real a los resultados de una consulta. Te suscribes a
consultas con el método `subscribe` de `ConvexClient`, que devuelve un `Flow`.
El contenido del `Flow` cambiará con el tiempo a medida que cambien los datos
subyacentes que respaldan la consulta.

Todos los métodos de `ConvexClient` son funciones `suspend` y deben llamarse desde un
`CoroutineScope` u otra función `suspend`. Una forma sencilla de consumir una
consulta que devuelve una lista de strings desde un `@Composable` es usar una
combinación de un estado mutable que contenga una lista y `LaunchedEffect`:

```kotlin
var workouts: List<String> by remember { mutableStateOf(listOf()) }
LaunchedEffect("onLaunch") {
    client.subscribe<List<String>>("workouts:get").collect { result ->
        result.onSuccess { receivedWorkouts ->
            workouts = receivedWorkouts
        }
    }
}
```

Cada vez que cambien los datos que alimentan la consulta de backend `"workouts:get"`, se emitirá un nuevo
`Result<List<String>>` en el `Flow` y la lista `workouts` se actualizará con los nuevos datos. Cualquier interfaz de usuario que use `workouts` se reconstruirá,
ofreciéndote una UI totalmente reactiva.

Nota: quizá prefieras encapsular la lógica de suscripción en un Repository, como
se describe en los
[patrones de arquitectura de Android](https://developer.android.com/topic/architecture/data-layer).

### Argumentos de consulta \{#query-arguments\}

Puedes pasar argumentos a `subscribe` y se pasarán a la función `query`
correspondiente en el backend. Los argumentos están tipados como
`Map<String, Any?>`. Los valores del mapa deben ser valores primitivos u otros
mapas y listas.

```kotlin
val favoriteColors = mapOf("favoriteColors" to listOf("blue", "red"))
client.subscribe<List<String>>("users:list", args = favoriteColors)
```

Asumiendo una consulta de backend que acepta un argumento `favoriteColors`, el valor puede
recibirse y usarse para aplicar lógica en la función de consulta.

<Admonition type="tip">
  Usa [Kotlin Data classes](/client/android/data-types.md#custom-data-types) serializables
  para convertir automáticamente objetos de Convex en clases de modelo de Kotlin.
</Admonition>

<Admonition type="caution">
  * Hay consideraciones importantes al
    [enviar y recibir números](/client/android/data-types.md#numerical-types)
    entre Kotlin y Convex.
  * `_` se usa para indicar campos privados en Kotlin. Si quieres usar los campos
    de Convex `_creationTime` y `_id` directamente sin advertencias, tendrás que
    [convertir el nombre del campo en Kotlin](/client/android/data-types.md#field-name-conversion).
  * Dependiendo de tus funciones de backend, puede que tengas que lidiar con
    [palabras clave reservadas de Kotlin](/client/android/data-types.md#field-name-conversion).
</Admonition>

### Duración de la suscripción \{#subscription-lifetime\}

El `Flow` devuelto por `subscribe` persistirá mientras haya algo
esperando consumir sus resultados. Cuando un `@Composable` o `ViewModel` con una
suscripción sale de su ámbito, la suscripción de consulta subyacente en Convex se
cancelará.

## Edición de datos \{#editing-data\}

Puedes usar el método `mutation` en `ConvexClient` para ejecutar una
[mutación](https://docs.convex.dev/functions/mutation-functions) en el backend.

Debes usarlo en otra función `suspend` o en un `CoroutineScope`.
Las mutaciones pueden devolver un valor o no. Si esperas un tipo en la respuesta,
indícalo en la firma de la llamada.

Las mutaciones también pueden recibir argumentos, igual que las consultas. Aquí tienes un ejemplo de
cómo devolver un tipo desde una mutación con argumentos:

```kotlin
val recordsDeleted = convex.mutation<@ConvexNum Int>(
  "messages:cleanup",
  args = mapOf("keepLatest" to 100)
)
```

Si se produce un error durante una llamada a `mutation`, se lanzará una excepción.
Suele ser conveniente capturar
[`ConvexError`](https://docs.convex.dev/functions/error-handling/application-errors)
y `ServerError` y gestionarlos de la forma que resulte más adecuada para tu aplicación.
Consulta la documentación sobre
[manejo de errores](https://docs.convex.dev/functions/error-handling/) para más
detalles.

## Llamar a APIs de terceros \{#calling-third-party-apis\}

Puedes usar el método `action` en `ConvexClient` para ejecutar una
[acción](https://docs.convex.dev/functions/actions) en el backend.

Las llamadas a `action` pueden aceptar argumentos, devolver valores y lanzar excepciones al igual
que las llamadas a `mutation`.

Aunque puedes llamar a acciones desde Android, no siempre es la opción adecuada.
Consulta la documentación de acciones para obtener recomendaciones sobre
[llamar a acciones desde clientes](https://docs.convex.dev/functions/actions#calling-actions-from-clients).

## Autenticación con Auth0 \{#authentication-with-auth0\}

Puedes usar `ConvexClientWithAuth` en lugar de `ConvexClient` para configurar
la autenticación con [Auth0](https://auth0.com/). Necesitarás la biblioteca
`convex-android-auth0` para hacerlo, así como una cuenta de Auth0 y la
configuración de la aplicación.

Consulta el
[README](https://github.com/get-convex/convex-android-auth0/blob/main/README.md)
en el repositorio `convex-android-auth0` para obtener instrucciones de
configuración más detalladas y la
[aplicación de ejemplo Workout](https://github.com/get-convex/android-convex-workout)
que está configurada para Auth0. La documentación general de
[autenticación de Convex](https://docs.convex.dev/auth) también es un buen recurso.

También es posible integrar otros proveedores de autenticación similares
basados en OpenID Connect. Consulta la interfaz
[`AuthProvider`](https://github.com/get-convex/convex-mobile/blob/5babd583631a7ff6d739e1a2ab542039fd532548/android/convexmobile/src/main/java/dev/convex/android/ConvexClient.kt#L291)
en el repositorio `convex-mobile` para obtener más información.

## Despliegues de producción y dev \{#production-and-dev-deployments\}

Cuando estés listo para avanzar hacia
[producción](https://docs.convex.dev/production) con tu aplicación, puedes configurar
tu sistema de compilación de Android para que diferentes compilaciones o variantes (flavors) de tu
aplicación apunten a distintos despliegues de Convex. Una forma bastante sencilla de hacerlo es
pasando distintos valores (por ejemplo, la URL de implementación) a diferentes destinos de compilación o
variantes.

Este es un ejemplo sencillo que muestra cómo usar diferentes URL de implementación para las compilaciones
de release y debug:

```kotlin
// In the android section of build.gradle.kts:
buildTypes {
    release {
        // Omite otras configuraciones como ProGuard ...
        resValue("string", "convex_url", "YOUR_PROD.convex.cloud")
    }

    debug {
        resValue("string", "convex_url", "YOUR_DEV.convex.cloud")
    }
}
```

Luego puedes construir tu `ConvexClient` usando un único recurso en el código, y así obtendrá el valor correcto en tiempo de compilación.

```kotlin
val convex = ConvexClient(context.getString(R.string.convex_url))
```

<Admonition type="tip">
  Puede que no quieras que estas URL se suban a tu repositorio. Un patrón común es
  crear un archivo personalizado `my_app.properties` que esté configurado para ignorarse en
  tu archivo `.gitignore`. Luego puedes leer este archivo en tu archivo `build.gradle.kts`.
  Puedes ver este patrón en uso en la
  [aplicación de ejemplo de entrenamiento](https://github.com/get-convex/android-convex-workout?tab=readme-ov-file#configuration).
</Admonition>

## Cómo estructurar tu aplicación \{#structuring-your-application\}

Los ejemplos mostrados en esta guía están pensados para ser breves y no ofrecen
orientación sobre cómo estructurar una aplicación completa.

La documentación oficial sobre
[arquitectura de aplicaciones Android](https://developer.android.com/topic/architecture/intro)
cubre las prácticas recomendadas para desarrollar aplicaciones, y Convex también tiene una
[aplicación de ejemplo de código abierto](https://github.com/get-convex/android-convex-workout/tree/main)
que muestra cómo podría ser una pequeña aplicación con varias pantallas.

En general, haz lo siguiente:

1. Adopta Flows y el
   [flujo de datos unidireccional](https://developer.android.com/develop/ui/compose/architecture#udf)
2. Define una
   [capa de datos](https://developer.android.com/topic/architecture/data-layer)
   clara (usa clases Repository con `ConvexClient` como tu fuente de datos)
3. Mantén el estado de la UI en un
   [ViewModel](https://developer.android.com/topic/architecture/recommendations#viewmodel)

## Pruebas \{#testing\}

`ConvexClient` es una clase `open`, por lo que se puede simular (mockear) en pruebas unitarias. Si
quieres usar más funcionalidad del cliente real, puedes pasar una
implementación falsa de `MobileConvexClientInterface` al constructor de `ConvexClient`. Solo ten
en cuenta que tendrás que proporcionar JSON en el
[formato JSON no documentado](https://github.com/get-convex/convex-mobile/blob/5babd583631a7ff6d739e1a2ab542039fd532548/android/convexmobile/src/main/java/dev/convex/android/jsonhelpers.kt#L47) de Convex.

También puedes usar el `ConvexClient` completo en pruebas de instrumentación de Android. Puedes configurar una instancia de backend específica para pruebas o ejecutar un servidor local de Convex y realizar pruebas de integración completas.

## Bajo el capó \{#under-the-hood\}

Convex para Android está basado en el
[cliente oficial de Convex para Rust](https://docs.convex.dev/client/rust). Se encarga de
mantener una conexión WebSocket con el backend de Convex e implementa el
protocolo completo de Convex.

Todas las llamadas a métodos en `ConvexClient` se gestionan mediante un runtime asincrónico de Tokio en el
lado de Rust y es seguro realizarlas desde el hilo principal de tu aplicación.

`ConvexClient` también hace un uso intensivo del
[framework de serialización de Kotlin](https://github.com/Kotlin/kotlinx.serialization/blob/master/docs/serialization-guide.md),
y la mayor parte de la funcionalidad de ese framework está disponible para que la uses en
tus aplicaciones. Internamente, `ConvexClient` habilita las opciones de JSON
[`ignoreUnknownKeys`](https://github.com/Kotlin/kotlinx.serialization/blob/master/docs/json.md#ignoring-unknown-keys)
y
[`allowSpecialFloatingPointValues`](https://github.com/Kotlin/kotlinx.serialization/blob/master/docs/json.md#allowing-special-floating-point-values).