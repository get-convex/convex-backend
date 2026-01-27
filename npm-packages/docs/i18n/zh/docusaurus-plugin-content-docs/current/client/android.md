---
title: "Android Kotlin"
sidebar_label: "Android Kotlin"
sidebar_position: 600
description:
  "适用于使用 Convex 的移动应用的 Android Kotlin 客户端库"
---

Convex Android 客户端库使你的 Android 应用能够与 Convex 后端交互。它允许你的前端代码：

1. 调用你的 [查询](/functions/query-functions.mdx)、[变更](/functions/mutation-functions.mdx) 和 [操作](/functions/actions.mdx)
2. 使用 [Auth0](/auth/auth0.mdx) 验证用户身份

该库是开源的，
[可在 GitHub 上获取](https://github.com/get-convex/convex-mobile/tree/main/android)。

请按照 [Android 快速开始](/quickstart/android.mdx) 开始使用。

## 安装 \{#installation\}

你需要对你的应用的 `build.gradle[.kts]` 文件做如下修改。

```kotlin
plugins {
    // ... existing plugins
    kotlin("plugin.serialization") version "1.9.0"
}

dependencies {
    // ... 现有依赖项
    implementation("dev.convex:android-convexmobile:0.4.1@aar") {
        isTransitive = true
    }
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.3")
}
```

之后，同步 Gradle 以使这些更改生效。你的应用现在就可以使用 Convex for Android 库，以及用于在你的代码与 Convex 后端之间进行通信的 Kotlin JSON 序列化功能。

## 连接到后端 \{#connecting-to-a-backend\}

`ConvexClient` 用于在你的应用和 Convex 后端之间建立并维护连接。首先，你需要创建一个客户端实例，并为其提供后端的部署 URL：

```kotlin
package com.example.convexapp

import dev.convex.android.ConvexClient

val convex = ConvexClient("https://<你的域名>.convex.cloud")
```

在整个应用进程的生命周期内，你应该只创建并使用一个 `ConvexClient` 实例。一个方便的做法是创建自定义 Android
[`Application`](https://developer.android.com/reference/android/app/Application)
子类，并在其中进行初始化：

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

完成上述步骤后，你就可以在 Jetpack Compose 的
`@Composable` 函数中像下面这样使用该客户端：

```kotlin
val convex = (application as MyApplication).convex
```

## 获取数据 \{#fetching-data\}

Android 版 Convex 为你提供对 Convex
[reactor](https://docs.convex.dev/tutorial/reactor) 的使用能力，它支持对查询结果进行实时
*订阅*。你可以通过 `ConvexClient` 上的 `subscribe` 方法订阅查询，该方法返回一个 `Flow`。随着支撑该查询的底层数据发生变化，`Flow` 中的内容也会随之不断更新。

`ConvexClient` 上的所有方法都是挂起函数，需要从 `CoroutineScope` 或其他 `suspend` 函数中调用。要在一个返回字符串列表的 `@Composable` 中使用查询结果，一个简单的方式是结合使用包含该列表的可变状态和 `LaunchedEffect`：

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

每当驱动后端 `"workouts:get"` 查询的数据发生变化时，新的
`Result<List<String>>` 就会被推送到 `Flow` 中，`workouts` 列表也会随之用新数据刷新。任何使用 `workouts` 的 UI 随后都会重新构建，从而为你提供一个完全响应式的 UI。

注意：你可能更倾向于将订阅逻辑封装到一个 Repository 中，如
[Android 架构模式](https://developer.android.com/topic/architecture/data-layer) 中所述。

### 查询参数 \{#query-arguments\}

你可以向 `subscribe` 传入参数，它们会被传递给关联的后端 `query` 函数。参数的类型是
`Map<String, Any?>`。map 中的值必须是原始类型的值，或者其他 map 或 list。

```kotlin
val favoriteColors = mapOf("favoriteColors" to listOf("blue", "red"))
client.subscribe<List<String>>("users:list", args = favoriteColors)
```

假设有一个后端查询接受 `favoriteColors` 参数，那么该值可以在查询函数中接收并用于执行相应逻辑。

<Admonition type="tip">
  使用可序列化的 [Kotlin Data classes](/client/android/data-types.md#custom-data-types)
  来自动将 Convex 对象转换为 Kotlin 模型类。
</Admonition>

<Admonition type="caution">
  * 在 Kotlin 与 Convex 之间
    [发送和接收数字](/client/android/data-types.md#numerical-types)
    时有一些重要的注意事项需要留意。
  * `_` 在 Kotlin 中通常用来表示私有字段。如果你想直接使用
    `_creationTime` 和 `_id` 这些 Convex 字段而不触发警告，你需要
    [在 Kotlin 中转换字段名](/client/android/data-types.md#field-name-conversion)。
  * 根据你的后端函数实现，你可能需要处理
    [Kotlin 保留关键字](/client/android/data-types.md#field-name-conversion)。
</Admonition>

### 订阅生命周期 \{#subscription-lifetime\}

从 `subscribe` 返回的 `Flow` 只要仍有代码在等待消费其结果，就会一直存在。当包含订阅的 `@Composable` 或 `ViewModel` 超出其作用域时，底层到 Convex 的查询订阅就会被取消。

## 编辑数据 \{#editing-data\}

你可以在 `ConvexClient` 上使用 `mutation` 方法来触发后端
[变更](https://docs.convex.dev/functions/mutation-functions)。

你需要在其他 `suspend` 函数或 `CoroutineScope` 中调用它。
变更函数可以返回值，也可以不返回。如果你期望响应中包含某种类型的值，
请在调用的函数签名中标明。

变更函数也可以像查询一样接收参数。下面是一个示例，展示了带参数的变更函数如何返回特定类型的值：

```kotlin
val recordsDeleted = convex.mutation<@ConvexNum Int>(
  "messages:cleanup",
  args = mapOf("keepLatest" to 100) // 保留最新的 100 条
)
```

如果在调用 `mutation` 时发生错误，会抛出异常。
通常你会希望捕获
[`ConvexError`](https://docs.convex.dev/functions/error-handling/application-errors)
和 `ServerError`，并在应用中以合适的方式进行处理。
有关更多详情，请参阅
[错误处理](https://docs.convex.dev/functions/error-handling/) 文档。

## 调用第三方 API \{#calling-third-party-apis\}

你可以在 `ConvexClient` 上使用 `action` 方法来触发后端的
[action](https://docs.convex.dev/functions/actions) 操作函数。

对 `action` 的调用可以像对 `mutation` 的调用一样接受参数、返回值并抛出异常。

即使你可以在 Android 客户端中调用操作函数，这样做也不一定总是合适的选择。
请参阅操作函数文档，了解
[从客户端调用操作函数](https://docs.convex.dev/functions/actions#calling-actions-from-clients) 的相关建议。

## 使用 Auth0 进行认证 \{#authentication-with-auth0\}

你可以使用 `ConvexClientWithAuth` 替代 `ConvexClient` 来配置基于
[Auth0](https://auth0.com/) 的身份认证。为此你需要 `convex-android-auth0`
库，以及一个 Auth0 账户和相应的应用配置。

更详细的设置说明请参阅
`convex-android-auth0` 仓库中的
[README](https://github.com/get-convex/convex-android-auth0/blob/main/README.md)，以及已经为 Auth0 配置完成的
[Workout 示例应用](https://github.com/get-convex/android-convex-workout)。
更通用的
[Convex 身份认证文档](https://docs.convex.dev/auth)
也是一个很好的参考资源。

也可以集成其他类似的 OpenID Connect
身份认证提供商。更多信息请参阅 `convex-mobile` 仓库中的
[`AuthProvider`](https://github.com/get-convex/convex-mobile/blob/5babd583631a7ff6d739e1a2ab542039fd532548/android/convexmobile/src/main/java/dev/convex/android/ConvexClient.kt#L291)
接口。

## 生产环境和开发环境部署 \{#production-and-dev-deployments\}

当你准备让应用迈向
[生产环境](https://docs.convex.dev/production) 时，你可以配置
Android 构建系统，让应用的不同构建或不同 flavor 变体
指向不同的 Convex 部署。一种相对简单的做法是，
为不同的构建目标或 flavor 传入不同的值（例如部署 URL）。

下面是一个简单示例，展示了在 release 和 debug 构建中使用不同的部署 URL：

```kotlin
// In the android section of build.gradle.kts:
buildTypes {
    release {
        // 省略 ProGuard 等其他配置 ...
        resValue("string", "convex_url", "YOUR_PROD.convex.cloud")
    }

    debug {
        resValue("string", "convex_url", "YOUR_DEV.convex.cloud")
    }
}
```

然后你就可以在代码中使用单个资源来构建你的 `ConvexClient`，并且它会在编译时自动获取到正确的值。

```kotlin
val convex = ConvexClient(context.getString(R.string.convex_url))
```

<Admonition type="tip">
  你可能不希望将这些 URL 提交到代码仓库中。一个常见的做法是
  创建一个自定义的 `my_app.properties` 文件，并在
  `.gitignore` 文件中配置忽略它。然后你可以在 `build.gradle.kts`
  文件中读取这个文件。你可以在
  [workout 示例应用](https://github.com/get-convex/android-convex-workout?tab=readme-ov-file#configuration)
  中看到这种模式的实际用法。
</Admonition>

## 组织你的应用结构 \{#structuring-your-application\}

本指南中的示例都比较简短，并没有提供如何为整个应用设计结构的指导。

官方的
[Android 应用架构](https://developer.android.com/topic/architecture/intro)
文档介绍了构建应用程序的最佳实践，Convex 也提供了一个
[开源示例应用](https://github.com/get-convex/android-convex-workout/tree/main)，
用于展示一个小型多界面应用可能的样子。

通常，可以遵循以下几点：

1. 拥抱 Flow 和
   [单向数据流](https://developer.android.com/develop/ui/compose/architecture#udf)
2. 设计清晰的
   [数据层](https://developer.android.com/topic/architecture/data-layer)
   （使用以 `ConvexClient` 作为数据源的 Repository 类）
3. 在
   [ViewModel](https://developer.android.com/topic/architecture/recommendations#viewmodel)
   中持有 UI 状态

## 测试 \{#testing\}

`ConvexClient` 是一个 `open` 类，因此可以在单元测试中进行 mock 或 fake。如果
你想尽可能多地使用真实客户端的行为，可以在调用 `ConvexClient` 构造函数时传入一个假的
`MobileConvexClientInterface`。但要注意，你需要提供符合 Convex 尚未公开文档说明的
[JSON 格式](https://github.com/get-convex/convex-mobile/blob/5babd583631a7ff6d739e1a2ab542039fd532548/android/convexmobile/src/main/java/dev/convex/android/jsonhelpers.kt#L47) 的 JSON 数据。

你也可以在 Android instrumentation tests 中使用完整的 `ConvexClient`。你可以
为测试配置一个专用的后端实例，或者运行本地 Convex 服务器并执行完整的集成测试。

## 底层原理 \{#under-the-hood\}

Android 版 Convex 构建在官方的
[Convex Rust client](https://docs.convex.dev/client/rust) 之上。它负责维护与 Convex 后端的 WebSocket 连接，并实现完整的 Convex 协议。

对 `ConvexClient` 的所有方法调用都通过 Rust 端的 Tokio 异步运行时处理，并且可以安全地从应用的主线程调用。

`ConvexClient` 还大量使用
[Kotlin 的序列化框架](https://github.com/Kotlin/kotlinx.serialization/blob/master/docs/serialization-guide.md)，
该框架的大部分功能都可以供你在应用中使用。在内部实现中，`ConvexClient` 为 JSON 启用了
[`ignoreUnknownKeys`](https://github.com/Kotlin/kotlinx.serialization/blob/master/docs/json.md#ignoring-unknown-keys)
和
[`allowSpecialFloatingPointValues`](https://github.com/Kotlin/kotlinx.serialization/blob/master/docs/json.md#allowing-special-floating-point-values)
这两个特性。