---
title: "Swift 与 Convex 类型转换"
sidebar_label: "数据类型"
hidden: false
sidebar_position: 5
description: "在 Swift 应用与 Convex 之间进行类型自定义和转换"
---

## 自定义数据类型 \{#custom-data-types\}

Convex 允许你在后端使用 TypeScript 对象轻松表示数据，
并可以从查询、变更和操作中返回这些对象。要在 Swift 端处理这些对象，
请创建遵循 `Decodable` 协议的 `struct` 定义。通常这非常简单，
因为只要一个 `struct` 的所有成员都是 `Decodable`，它就可以自动遵循该协议。

设想一个 Convex 查询函数会返回如下 JavaScript 对象形式的结果：

```tsx
{
  name: "Guardians",
  uniformColors: ["blue", "white", "red"],
  wins: 80n,
  losses: 60n
}
```

在 Swift 中可以表示为：

```swift
struct BaseballTeam: Decodable {
  let name: String
  let uniformColors: [String]
  @ConvexInt
  var wins: Int
  @ConvexInt
  var losses: Int
}
```

然后你可以在 `subscribe` 调用中将该类型作为 yielding 参数传入：

```swift
convex.subscribe(to: "mlb:first_place_team",
               with: ["division": "AL Central"],
           yielding: BaseballTeam.self)
```

远程函数返回的数据会被反序列化为你的自定义结构体。
通常，你要使用的类型可以从调用上下文中推断出来，因此可以
省略 `yielding` 参数。

## 数值类型 \{#numerical-types\}

像 `Int` 和 `Double` 这样的数值类型会以一种特殊格式进行编码，以确保
能够与你的 TypeScript 后端函数正确互操作。要在 Swift 侧安全地使用它们，
请务必使用以下属性包装器之一。

| Type                           | Wrapper                |
| ------------------------------ | ---------------------- |
| `Float` or `Double`            | `@ConvexFloat`         |
| `Float?` or `Double?`          | `@OptionalConvexFloat` |
| `Int` or `Int32` or `Int64`    | `@ConvexInt`           |
| `Int?` or `Int32?` or `Int64?` | `@OptionalConvexInt`   |

请注意，带有包装器的 `struct` 属性必须声明为 `var`。

## 字段名转换 \{#field-name-conversion\}

如果你的代码接收到的对象字段名需要或希望转换成不同的名字，你可以使用 `CodingKeys` `enum` 来指定远程字段名与结构体字段名之间的映射。比如，设想一个后端函数或
API 返回如下的日志条目，用来表示某人在何时进出：

```tsx
{name: "Bob", in: "2024-10-03 08:00:00", out: "2024-10-03 11:00:00"}
```

这些数据不能直接解码为一个 `struct`，因为 `in` 是 Swift 中的关键字。我们可以使用 `CodingKeys` 为它指定一个替代名称，同时仍然从原始名称对应的字段中读取数据。

```swift
struct Log: Decodable {
  let name: String
  let inTime: String
  let outTime: String

  enum CodingKeys: String, CodingKey {
    case name
    case inTime = "in"
    case outTime = "out"
  }
}
```

## 综合运用 \{#putting-it-all-together\}

在上面的自定义数据类型示例中，通过在后端数据里的 `wins` 和 `losses` 值后面加上后缀 `n` 来使用 JavaScript 的 `BigInt` 类型，从而让 Swift 代码可以使用 `Int`。如果代码改为使用普通的 JavaScript `number` 类型，在 Swift 端接收到的将是浮点数值，反序列化为 `Int` 就会失败。

如果你遇到类似情况：`number` 被使用，但按约定它只包含整数值，你可以在自己的 `struct` 中通过字段名映射和自定义属性来处理这种情况，从而屏蔽底层的浮点数表示形式。

```swift
struct BaseballTeam: Decodable {
  let name: String
  let uniformColors: [String]
  @ConvexFloat
  private var internalWins: Double
  @ConvexFloat
  private var internalLosses: Double

  enum CodingKeys: String, CodingKey {
    case name
    case uniformColors
    case internalWins = "wins"
    case internalLosses = "losses"
  }

  // 将 Double 值公开为 Int
  var wins: Int { Int(internalWins) }
  var losses: Int { Int(internalLosses) }
}
```

这种做法是将 `Double` 值以私有方式保存，并使用与后端值不同的名称，
然后再添加自定义属性来提供 `Int` 值。
