---
title: "Android Kotlin"
sidebar_label: "Android Kotlin"
sidebar_position: 600
---

Convex Android client library enables your Android application to interact with
your Convex backend. It allows your frontend code to:

1. Call
   your [queries](/docs/functions/query-functions.mdx), [mutations](/docs/functions/mutation-functions.mdx) and [actions](/docs/functions/actions.mdx)
2. Authenticate users using [Auth0](/docs/auth/auth0.mdx)

The library is open source and
[available on GitHub](https://github.com/get-convex/convex-mobile/tree/main/android).

Follow the [Android Quickstart](/docs/quickstart/android.mdx) to get started.

## Installation

You'll need to make the following changes to your app's `build.gradle[.kts]`
file.

```kotlin
plugins {
    // ... existing plugins
    kotlin("plugin.serialization") version "1.9.0"
}

dependencies {
    // ... existing dependencies
    implementation("dev.convex:android-convexmobile:0.4.1@aar") {
        isTransitive = true
    }
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.3")
}
```

After that, sync Gradle to pick up those changes. Your app will now have access
to the Convex for Android library as well as Kotlin's JSON serialization which
is used to communicate between your code and the Convex backend.

## Connecting to a backend

The `ConvexClient` is used to establish and maintain a connect between your
application and the Convex backend. First you need to create an instance of the
client by giving it your backend deployment URL:

```kotlin
package com.example.convexapp

import dev.convex.android.ConvexClient

val convex = ConvexClient("https://<your domain here>.convex.cloud")
```

You should create and use one instance of the `ConvexClient` for the lifetime of
your application process. It can be convenient to create a custom Android
[`Application`](https://developer.android.com/reference/android/app/Application)
subclass and initialize it there:

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

Once you've done that, you can access the client from a Jetpack Compose
`@Composable` function like this:

```kotlin
val convex = (application as MyApplication).convex
```

## Fetching data

Convex for Android gives you access to the Convex
[reactor](https://docs.convex.dev/tutorial/reactor), which enables real-time
_subscriptions_ to query results. You subscribe to queries with the `subscribe`
method on `ConvexClient` which returns a `Flow`. The contents of the `Flow` will
change over time as the underlying data backing the query changes.

All methods on `ConvexClient` suspend, and need to be called from a
`CoroutineScope` or another `suspend` function. A simple way to consume a query
that returns a list of strings from a `@Composable` is to use a combination of
mutable state containing a list and `LaunchedEffect`:

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

Any time the data that powers the backend `"workouts:get"` query changes, a new
`Result<List<String>>` will be emitted into the `Flow` and the `workouts` list
will refresh with the new data. Any UI that uses `workouts` will then rebuild,
giving you a fully reactive UI.

Note: you may prefer to put the subscription logic wrapped a Repository as
described in the
[Android architecture patterns](https://developer.android.com/topic/architecture/data-layer).

### Query arguments

You can pass arguments to `subscribe` and they will be supplied to the
associated backend `query` function. The arguments are typed as
`Map<String, Any?>`. The values in the map must be primitive values or other
maps and lists.

```kotlin
val favoriteColors = mapOf("favoriteColors" to listOf("blue", "red"))
client.subscribe<List<String>>("users:list", args = favoriteColors)
```

Assuming a backend query that accepts a `favoriteColors` argument, the value can
be received and used to perform logic in the query function.

<Admonition type="tip">
Use serializable [Kotlin Data classes](/docs/client/android/data-types.md#custom-data-types)
to automatically convert Convex objects to Kotlin model classes.
</Admonition>

<Admonition type="caution">
* There are important gotchas when
  [sending and receiving numbers](/docs/client/android/data-types.md#numerical-types)
  between Kotlin and Convex.
* `_` is a used to signify private fields in Kotlin. If you want to use a
  `_creationTime` and `_id` Convex fields directly without warnings you'll have
  to
  [convert the field name in Kotlin](/docs/client/android/data-types.md#field-name-conversion).
* Depending on your backend functions, you may need to deal with
  [reserved Kotlin keywords](/docs/client/android/data-types.md#field-name-conversion).
</Admonition>

### Subscription lifetime

The `Flow` returned from `subscribe` will persist as long as something is
waiting to consume results from it. When a `@Composable` or `ViewModel` with a
subscription goes out of scope, the underlying query subscription to Convex will
be canceled.

## Editing data

You can use the `mutation` method on `ConvexClient` to trigger a backend
[mutation](https://docs.convex.dev/functions/mutation-functions).

You'll need to use it in another `suspend` function or a `CoroutineScope`.
Mutations can return a value or not. If you expect a type in the response,
indicate it in the call signature.

Mutations can also receive arguments, just like queries. Here's an example of
returning a type from a mutation with arguments:

```kotlin
val recordsDeleted = convex.mutation<@ConvexNum Int>(
  "messages:cleanup",
  args = mapOf("keepLatest" to 100)
)
```

If an error occurs during a call to `mutation`, it will throw an exception.
Typically you may want to catch
[`ConvexError`](https://docs.convex.dev/functions/error-handling/application-errors)
and `ServerError` and handle them however is appropriate in your application.
See documentation on
[error handling](https://docs.convex.dev/functions/error-handling/) for more
details.

## Calling third-party APIs

You can use the `action` method on `ConvexClient` to trigger a backend
[action](https://docs.convex.dev/functions/actions).

Calls to `action` can accept arguments, return values and throw exceptions just
like calls to `mutation`.

Even though you can call actions from Android, it's not always the right choice.
See the action docs for tips on
[calling actions from clients](https://docs.convex.dev/functions/actions#calling-actions-from-clients).

## Authentication with Auth0

You can use `ConvexClientWithAuth` in place of `ConvexClient` to configure
authentication with [Auth0](https://auth0.com/). You'll need the
`convex-android-auth0` library to do that, as well as an Auth0 account and
application configuration.

See the
[README](https://github.com/get-convex/convex-android-auth0/blob/main/README.md)
in the `convex-android-auth0` repo for more detailed setup instructions, and the
[Workout example app](https://github.com/get-convex/android-convex-workout)
which is configured for Auth0. The overall
[Convex authentication docs](https://docs.convex.dev/auth) are a good resource
as well.

It should also be possible to integrate other similar OpenID Connect
authentication providers. See the
[`AuthProvider`](https://github.com/get-convex/convex-mobile/blob/5babd583631a7ff6d739e1a2ab542039fd532548/android/convexmobile/src/main/java/dev/convex/android/ConvexClient.kt#L291)
interface in the `convex-mobile` repo for more info.

## Production and dev deployments

When you're ready to move toward
[production](https://docs.convex.dev/production) for your app, you can setup
your Android build system to point different builds or flavors of your
application to different Convex deployments. One fairly simple way to do it is
by passing different values (e.g. deployment URL) to different build targets or
flavors.

Here's a simple example that shows using different deployment URLs for release
and debug builds:

```kotlin
// In the android section of build.gradle.kts:
buildTypes {
    release {
        // Snip various other config like ProGuard ...
        resValue("string", "convex_url", "YOUR_PROD.convex.cloud")
    }

    debug {
        resValue("string", "convex_url", "YOUR_DEV.convex.cloud")
    }
}
```

Then you can build your `ConvexClient` using a single resource in code, and it
will get the right value at compile time.

```kotlin
val convex = ConvexClient(context.getString(R.string.convex_url))
```

<Admonition type="tip">
You may not want these urls checked into your repository. One pattern is to 
create a custom `my_app.properties` file that is configured to be ignored in
your `.gitignore` file. You can then read this file in your `build.gradle.kts` 
file. You can see this pattern in use in the
[workout sample app](https://github.com/get-convex/android-convex-workout?tab=readme-ov-file#configuration).
</Admonition>

## Structuring your application

The examples shown in this guide are intended to be brief, and don't provide
guidance on how to structure a whole application.

The official
[Android application architecture](https://developer.android.com/topic/architecture/intro)
docs cover best practices for building applications, and Convex also has a
[sample open source application](https://github.com/get-convex/android-convex-workout/tree/main)
that attempts to demonstrate what a small multi-screen application might look
like.

In general, do the following:

1. Embrace Flows and
   [unidirectional data flow](https://developer.android.com/develop/ui/compose/architecture#udf)
2. Have a clear
   [data layer](https://developer.android.com/topic/architecture/data-layer)
   (use Repository classes with `ConvexClient` as your data source)
3. Hold UI state in a
   [ViewModel](https://developer.android.com/topic/architecture/recommendations#viewmodel)

## Testing

`ConvexClient` is an `open` class so it can be mocked or faked in unit tests. If
you want to use more of the real client, you can pass a fake
`MobileConvexClientInterface` in to the `ConvexClient` constructor. Just be
aware that you'll need to provide JSON in Convex's undocumented
[JSON format](https://github.com/get-convex/convex-mobile/blob/5babd583631a7ff6d739e1a2ab542039fd532548/android/convexmobile/src/main/java/dev/convex/android/jsonhelpers.kt#L47).

You can also use the full `ConvexClient` in Android instrumentation tests. You
can setup a special backend instance for testing or run a local Convex server
and run full integration tests.

## Under the hood

Convex for Android is built on top of the official
[Convex Rust client](https://docs.convex.dev/client/rust). It handles
maintaining a WebSocket connection with the Convex backend and implements the
full Convex protocol.

All method calls on `ConvexClient` are handled via a Tokio async runtime on the
Rust side and are safe to call from the application's main thread.

`ConvexClient` also makes heavy use of
[Kotlin's serialization framework](https://github.com/Kotlin/kotlinx.serialization/blob/master/docs/serialization-guide.md),
and most of the functionality in that framework is available for you to use in
your applications. Internally, `ConvexClient` enables the JSON
[`ignoreUnknownKeys`](https://github.com/Kotlin/kotlinx.serialization/blob/master/docs/json.md#ignoring-unknown-keys)
and
[`allowSpecialFloatingPointValues`](https://github.com/Kotlin/kotlinx.serialization/blob/master/docs/json.md#allowing-special-floating-point-values)
features.
