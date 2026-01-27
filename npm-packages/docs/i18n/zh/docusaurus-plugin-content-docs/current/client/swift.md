---
title: "iOS 与 macOS Swift"
sidebar_label: "Swift"
sidebar_position: 700
description: "适用于使用 Convex 的 iOS 和 macOS 应用的 Swift 客户端库"
---

Convex Swift 客户端库使你的 iOS 或 macOS 应用能够
与 Convex 后端交互。它可以让你的前端代码：

1. 调用你的 [查询](/functions/query-functions.mdx)、[变更](/functions/mutation-functions.mdx) 和 [操作](/functions/actions.mdx)
2. 使用 [Auth0](/auth/auth0.mdx) 对用户进行身份验证

该库是开源的，
并已[在 GitHub 上发布](https://github.com/get-convex/convex-swift)。

请按照 [Swift 快速入门](/quickstart/swift.mdx) 开始上手。

## 安装 \{#installation\}

对于使用 Xcode 的 iOS 或 macOS 项目，你需要执行以下步骤来添加对 `ConvexMobile` 库的依赖。

1. 在左侧项目导航器中点击顶层的 app 容器

2. 在 PROJECT 标题下点击应用名称

3. 点击 *Package Dependencies* 选项卡

4. 点击 + 按钮

   ![Screenshot 2024-10-02 at 2.33.43 PM.png](/screenshots/swift_qs_step_2.png)

5. 将
   [`https://github.com/get-convex/convex-swift`](https://github.com/get-convex/convex-swift)
   粘贴到搜索框中并按下回车键

6. 当 `convex-swift` 包加载完成后，点击 Add Package 按钮

7. 在 *Package Products* 对话框中，在 *Add to Target* 下拉框里选择你的目标（Target）名称

8. 点击 *Add Package*

## 连接到后端 \{#connecting-to-a-backend\}

`ConvexClient` 用于在你的应用程序和 Convex 后端之间建立并维护连接。首先，你需要通过提供后端的部署 URL 来创建一个客户端实例：

```swift
import ConvexMobile

let convex = ConvexClient(deploymentUrl: "https://<your domain here>.convex.cloud")
```

在整个应用进程的生命周期内，你应该只创建并使用一个 `ConvexClient` 实例。你可以像上面所示那样，把这个 client 存在一个全局常量中。与 Convex 后端的实际连接只有在你调用 `ConvexClient` 实例上的某个方法时才会发起。之后它会保持这条连接，并在连接中断时自动重新建立。

## 获取数据 \{#fetching-data\}

Swift Convex 库让你可以使用 Convex 同步引擎，它支持对查询结果进行实时 *订阅*。你可以在 `ConvexClient` 上调用 `subscribe` 方法来订阅查询，该方法会返回一个 [`Publisher`](https://developer.apple.com/documentation/combine)。随着支撑该查询的底层数据发生变化，通过 `Publisher` 提供的数据也会随时间变化。

你可以在 `Publisher` 上调用方法来转换并消费其提供的数据。

在 `View` 中消费一个返回字符串列表的查询，有一种简单的方式：使用包含列表的 `@State`，配合 `.task` 修饰符，并在其中编写将查询结果作为 `AsyncSequence` 进行循环的代码：

```swift
struct ColorList: View {
  @State private var colors: [String] = []

  var body: some View {
    List {
      ForEach(colors, id: \.self) { color in
        Text(color)
      }
    }.task {
      let latestColors = convex.subscribe(to: "colors:get", yielding: [String].self)
        .replaceError(with: [])
        .values
      for await colors in latestColors {
        self.colors = colors
      }
    }
  }
}
```

每当驱动后端 `"colors:get"` 查询的数据发生变化时，一个新的 `String` 数组就会出现在 `AsyncSequence` 中，并且 `View` 的 `colors` 列表会被赋值为这份新数据。然后 UI 会以响应式方式重新构建，以反映更新后的数据。

### 查询参数 \{#query-arguments\}

你可以向 `subscribe` 传入参数，这些参数会被传递给关联的后端 `query` 函数。参数必须是以字符串为键的 Dictionary，其值通常应为原始类型、Array 或其他 Dictionary。

```swift
let publisher = convex.subscribe(to: "colors:get",
                               with:["onlyFavorites": true],
                           yielding:[String].self)
```

假设 `colors:get` 查询接受一个 `onlyFavorites` 参数，则可以在查询函数中接收该值并据此执行逻辑。

<Admonition type="tip">
  使用 [Decodable 结构体](/client/swift/data-types.md#custom-data-types)
  将 Convex 对象自动转换为 Swift 结构体。
</Admonition>

<Admonition type="caution">
  * 在 Swift 和 Convex 之间
    [发送和接收数字](/client/swift/data-types.md#numerical-types)
    时有一些需要特别注意的重要问题。
  * 根据你的后端函数实现，你可能需要处理
    [Swift 保留关键字](/client/swift/data-types.md#field-name-conversion)。
</Admonition>

### 订阅生命周期 \{#subscription-lifetime\}

从 `subscribe` 返回的 `Publisher` 会在其关联的 `View` 或 `ObservableObject` 存在期间一直保持有效。当其中任意一个不再是 UI 的一部分时，针对 Convex 的底层查询订阅将被取消。

## 编辑数据 \{#editing-data\}

你可以在 `ConvexClient` 上使用 `mutation` 方法来触发后端的[变更函数](/functions/mutation-functions.mdx)。

`mutation` 是一个 `async` 方法，因此你需要在 `Task` 中调用它。
变更函数可以返回一个值，也可以不返回。

变更函数也可以像查询一样接收参数。下面是一个调用带参数的变更函数并返回值的示例：

```swift
let isColorAdded: Bool = try await convex.mutation("colors:put", with: ["color": newColor])
```

### 处理错误 \{#handling-errors\}

如果在调用 `mutation` 时发生错误，它会抛出异常。通常你可能会想要捕获
[`ConvexError`](/functions/error-handling/application-errors.mdx) 和 `ServerError`，
并根据你的应用需求进行适当处理。

下面是一个小示例，演示如果 `colors:put` 抛出了一个 `ConvexError`，且错误信息提示某个颜色已经存在时，你可能会如何处理这个错误。

```swift
do {
  try await convex.mutation("colors:put", with: ["color": newColor])
} catch ClientError.ConvexError(let data) {
  errorMessage = try! JSONDecoder().decode(String.self, from: Data(data.utf8))
  colorNotAdded = true
}
```

更多详情请参阅[错误处理](/functions/error-handling/)文档。

## 调用第三方 API \{#calling-third-party-apis\}

你可以在 `ConvexClient` 上使用 `action` 方法来触发后端的
[操作](/functions/actions.mdx)。

对 `action` 的调用可以像对 `mutation` 的调用一样接受参数、返回值并抛出异常。

尽管你可以在客户端代码中调用操作函数，但这并不总是最合适的选择。请参阅操作函数文档，了解
[从客户端调用操作函数](/functions/actions.mdx#calling-actions-from-clients) 的相关建议。

## 使用 Auth0 进行身份验证 \{#authentication-with-auth0\}

你可以使用 `ConvexClientWithAuth` 来替代 `ConvexClient`，以配置
通过 [Auth0](https://auth0.com/) 进行的身份验证。为此，你需要
`convex-swift-auth0` 库，以及一个 Auth0 账号和
相应的应用配置。

请查看
`convex-swift-auth0` 仓库中的 [README](https://github.com/get-convex/convex-swift-auth0/blob/main/README.md)，以获取更详细的设置说明，以及
已经为 Auth0 配置好的 [Workout 示例应用](https://github.com/get-convex/ios-convex-workout)。此外，[Convex 身份验证文档](/auth.mdx)
也是一个很好的参考资源。

通常也可以集成其他类似的 OpenID Connect
身份验证提供商。更多信息请参见
`convex-swift` 仓库中的 [`AuthProvider`](https://github.com/get-convex/convex-swift/blob/c47aea414c92db2ccf3a0fa4f9db8caf2029b032/Sources/ConvexMobile/ConvexMobile.swift#L188) 协议。

## 生产环境和开发环境（dev）部署 \{#production-and-dev-deployments\}

当你准备让应用迈向 [生产环境](/production.mdx) 时，
可以配置 Xcode 构建系统，让不同的构建目标指向不同的 Convex 部署。构建环境配置通常高度定制化，你或你的团队可能有不同的约定，但下面是一种可行的做法。

1. 在项目源码中创建 “Dev” 和 “Prod” 两个文件夹。
2. 在每个文件夹中添加一个 `Env.swift` 文件，内容类似于：

```swift
let deploymentUrl = "https://$DEV_OR_PROD.convex.cloud"
```

3. 将你的开发环境（dev）URL 写入 `Dev/Env.swift`，将生产环境（prod）URL 写入 `Prod/Env.swift`。
   即使 Xcode 提示 `deploymentUrl` 被多次定义，也不用担心。
4. 在左侧的资源管理器视图中点击你的顶级项目。
5. 在 **TARGETS** 列表中选择你的构建 target。
6. 将该 target 的名称修改为以 “dev” 结尾。
7. 右键单击（或按 Ctrl 点击）该 target 并选择复制，为副本取一个以 “prod” 结尾的名称。
8. 选中 “dev” target，点击 **Build Phases** 选项卡。
9. 展开 **Compile Sources** 部分。
10. 选中 `Prod/Env.swift`，点击 - 按钮将其移除。
11. 同样地，打开 “prod” target，并从它的 sources 中移除 `Dev/Env.swift`。

![截图 2024-10-03 at 1.34.34 PM.png](/screenshots/swift_env_setup.png)

现在，你可以在创建 `ConvexClient` 的地方引用 `deploymentUrl`，并且根据你构建的 target 不同，它会自动使用开发环境（dev）或生产环境（prod）的 URL。

## 组织你的应用结构 \{#structuring-your-application\}

本指南中的示例都比较简短，并没有提供如何组织完整应用的指导。

如果你想要一种更健壮、分层的做法，可以将与 `ConvexClient` 交互的代码放到一个符合 `ObservableObject` 的类中。然后你的 `View` 可以将该对象作为 `@StateObject` 进行观察，并在其发生变化时重新构建。

例如，如果我们将上面的 `colors:get` 示例改写成一个 `ViewModel: ObservableObject` 类，那么 `View` 将不再直接参与数据获取——它只知道 `colors` 列表由 `ViewModel` 提供。

```swift
import SwiftUI

class ViewModel: ObservableObject {
  @Published var colors: [String] = []

  init() {
    convex.subscribe(to: "colors:get")
      .replaceError(with: [])
      .receive(on: DispatchQueue.main)
      .assign(to: &$colors)
  }
}

struct ContentView: View {
  @StateObject var viewModel = ViewModel()

  var body: some View {
    List {
      ForEach(viewModel.colors, id: \.self) { color in
        Text(color)
      }
    }
  }
}
```

根据你的需求和应用的规模，像 https://github.com/nalexn/clean-architecture-swiftui 这样的示例所展示的那样，为其赋予更加规范的结构可能是合理的选择。

## 底层原理 \{#under-the-hood\}

Swift Convex 库构建在官方的 [Convex Rust 客户端](/client/rust.md) 之上。它负责维护与 Convex 后端的 WebSocket 连接，并实现完整的 Convex 协议。

所有对 `ConvexClient` 的方法调用都会通过 Rust 端的 Tokio 异步运行时处理，并且可以在应用的主 actor（main actor）中安全调用。