---
title: "Swift と Convex の型変換"
sidebar_label: "データ型"
hidden: false
sidebar_position: 5
description: "Swift アプリと Convex 間での型のカスタマイズと変換"
---

## カスタムデータ型 \{#custom-data-types\}

Convex では、バックエンドのデータを TypeScript オブジェクトとして簡単に表現でき、そのオブジェクトをクエリ、ミューテーション、アクションから返すことができます。Swift 側でオブジェクトを扱うには、`Decodable` プロトコルに準拠した `struct` 定義を作成します。通常これはかなり簡単で、すべてのメンバーが `Decodable` である任意の `struct` は自動的に準拠できます。

次のような JavaScript オブジェクトを返す Convex のクエリ関数を考えてみましょう。

```tsx
{
  name: "Guardians",
  uniformColors: ["blue", "white", "red"],
  wins: 80n,
  losses: 60n
}
```

Swift では次のように表現できます。

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

その型を、`subscribe` 呼び出しでコールバックに渡される引数の型として指定できます。

```swift
convex.subscribe(to: "mlb:first_place_team",
               with: ["division": "AL Central"],
           yielding: BaseballTeam.self)
```

リモート関数からのデータは、定義したカスタム構造体にデシリアライズされます。
多くの場合、その型の利用は呼び出し側のコンテキストから推論できるため、
`yield` に渡す引数を省略できます。

## 数値型 \{#numerical-types\}

`Int` や `Double` のような数値型は、TypeScript バックエンド関数との正しい相互運用性を確保するために、特別なフォーマットでエンコードされています。Swift 側で安全に使用するには、次のいずれかのプロパティラッパーを必ず使用してください。

| Type                           | Wrapper                |
| ------------------------------ | ---------------------- |
| `Float` or `Double`            | `@ConvexFloat`         |
| `Float?` or `Double?`          | `@OptionalConvexFloat` |
| `Int` or `Int32` or `Int64`    | `@ConvexInt`           |
| `Int?` or `Int32?` or `Int64?` | `@OptionalConvexInt`   |

ラッパー付きの `struct` のプロパティは、`var` として宣言する必要があることに注意してください。

## フィールド名の変換 \{#field-name-conversion\}

コードで、別の名前に変換する必要がある、あるいは変換したいフィールド名を持つオブジェクトを受け取る場合、`CodingKeys` `enum` を使って、リモート側の名前から構造体のプロパティ名へのマッピングを指定できます。たとえば、誰かが入室したときと退室したときを表す、次のようなログエントリを返すバックエンド関数や API を想像してみてください。

```tsx
{name: "Bob", in: "2024-10-03 08:00:00", out: "2024-10-03 11:00:00"}
```

そのデータは `in` が Swift のキーワードであるため、そのまま `struct` にデコードすることはできません。`CodingKeys` を使うことで、元の名前からデータを取り込みつつ、別の名前を割り当てることができます。

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

## すべてを組み合わせる \{#putting-it-all-together\}

前述のカスタムデータ型の例では、Swift のコードで `Int` を使えるようにするために、
バックエンド側のデータでは JavaScript の `BigInt` 型を使い、`wins` と `losses` の値の末尾に
`n` を付けています。一方で通常の JavaScript の `number` 型を使っていた場合には、
Swift 側ではそれらは浮動小数点数として受信され、`Int` へのデシリアライズは失敗します。

そのように `number` が使われているものの、慣例として整数のみを含む状況では、
`struct` 内でフィールド名の変換とカスタムプロパティを使うことで、
浮動小数点での表現を意識させないように処理できます。

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

  // Double値をIntとして公開する
  var wins: Int { Int(internalWins) }
  var losses: Int { Int(internalLosses) }
}
```

このパターンでは、バックエンドから渡される値とは異なる名前で、`Double` 型の値を非公開で保持します。
そのうえで、`Int` 型の値を提供するためのカスタムプロパティを追加します。
