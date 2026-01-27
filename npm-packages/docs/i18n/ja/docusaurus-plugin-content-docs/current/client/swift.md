---
title: "iOS & macOS 向け Swift"
sidebar_label: "Swift"
sidebar_position: 700
description: "Convex を利用する iOS および macOS アプリケーション向けの Swift クライアントライブラリ"
---

Convex の Swift クライアントライブラリを使用すると、iOS または macOS アプリケーションから
Convex のバックエンドと連携できます。これによりフロントエンドコードで次のことが可能になります:

1. [クエリ](/functions/query-functions.mdx)、[ミューテーション](/functions/mutation-functions.mdx)、[アクション](/functions/actions.mdx) を呼び出す
2. [Auth0](/auth/auth0.mdx) を使用してユーザーを認証する

このライブラリはオープンソースとして公開されており、
[GitHub で入手できます](https://github.com/get-convex/convex-swift)。

使い始めるには [Swift クイックスタート](/quickstart/swift.mdx) を参照してください。

## インストール \{#installation\}

Xcode の iOS または macOS プロジェクトで、`ConvexMobile` ライブラリを依存関係として追加するには、次の手順を実行します。

1. 左側のプロジェクトナビゲータで、最上位のアプリコンテナをクリックします

2. 「PROJECT」セクションの下にあるアプリ名をクリックします

3. *Package Dependencies* タブをクリックします

4. * ボタンをクリックします

   ![Screenshot 2024-10-02 at 2.33.43 PM.png](/screenshots/swift_qs_step_2.png)

5. 検索ボックスに
   [`https://github.com/get-convex/convex-swift`](https://github.com/get-convex/convex-swift)
   を貼り付けて Enter キーを押します

6. `convex-swift` パッケージが読み込まれたら、*Add Package* ボタンをクリックします

7. *Package Products* ダイアログで、*Add to Target* のドロップダウンメニューから自分のプロダクト名を選択します

8. *Add Package* をクリックします

## バックエンドへの接続 \{#connecting-to-a-backend\}

`ConvexClient` は、アプリケーションと Convex バックエンドとの間の接続を確立し、維持するために使用されます。まず、バックエンドのデプロイメントURLを指定して、このクライアントのインスタンスを作成します。

```swift
import ConvexMobile

let convex = ConvexClient(deploymentUrl: "https://あなたのドメイン.convex.cloud")
```

アプリケーションプロセスの存続期間を通して、`ConvexClient` のインスタンスは 1 つだけ作成して使用してください。上記のように、そのクライアントをグローバルな定数として保持できます。実際に Convex のバックエンドへの接続が確立されるのは、`ConvexClient` のメソッドを呼び出したタイミングです。その後は接続を維持し、切断された場合には再確立します。

## データの取得 \{#fetching-data\}

Swift Convex ライブラリを使うと Convex の同期エンジンにアクセスでき、
クエリ結果をリアルタイムに *サブスクリプション* できるようになります。
`ConvexClient` の `subscribe` メソッドでクエリを購読すると、
[`Publisher`](https://developer.apple.com/documentation/combine) が返されます。
この `Publisher` を通じて利用できるデータは、そのクエリの元になっている
データの変化に応じて時間とともに更新されます。

`Publisher` に対してメソッドを呼び出すことで、提供されるデータを変換したり
利用したりできます。

`View` の中で文字列のリストを返すクエリを扱う簡単な方法は、
リストを保持する `@State` と、クエリ結果を `AsyncSequence` としてループする
コードを含む `.task` モディファイアを組み合わせて使うことです。

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

バックエンドの `"colors:get"` クエリが参照しているデータが変更されるたびに、
新しい `String` 値の配列が `AsyncSequence` に流れてきて、
`View` の `colors` リストにその新しいデータが代入されます。すると UI は、
変更されたデータを反映するようにリアクティブに再構築されます。

### クエリ引数 \{#query-arguments\}

`subscribe` に引数を渡すことができ、その引数は対応するバックエンドの `query` 関数に渡されます。引数はキーが文字列の Dictionary である必要があり、値には一般的にプリミティブ型、配列（Array）、およびその他の Dictionary を使用します。

```swift
let publisher = convex.subscribe(to: "colors:get",
                               with:["onlyFavorites": true],
                           yielding:[String].self)
```

`colors:get` クエリが `onlyFavorites` 引数を受け取ると仮定すると、その値を受け取り、クエリ関数内での処理に利用できます。

<Admonition type="tip">
  Convex オブジェクトを Swift の struct に自動的に変換するには、
  [Decodable structs](/client/swift/data-types.md#custom-data-types)
  を使用してください。
</Admonition>

<Admonition type="caution">
  * Swift と Convex 間で
    [数値を送受信する](/client/swift/data-types.md#numerical-types)
    際には、いくつかの重要な注意点があります。
  * バックエンド関数の実装によっては、
    [Swift の予約語](/client/swift/data-types.md#field-name-conversion)
    を扱う必要がある場合があります。
</Admonition>

### サブスクリプションのライフタイム \{#subscription-lifetime\}

`subscribe` から返される `Publisher` は、関連する `View` または `ObservableObject` が UI 上に存在する限り存続します。いずれかが UI の一部でなくなった時点で、Convex への背後のクエリサブスクリプションは解除されます。

## データの編集 \{#editing-data\}

`ConvexClient` の `mutation` メソッドを使って、
バックエンドの[ミューテーション](/functions/mutation-functions.mdx)を実行できます。

`mutation` は `async` メソッドなので、`Task` 内で呼び出す必要があります。
ミューテーションは値を返しても返さなくても構いません。

ミューテーションはクエリと同様に引数を受け取ることもできます。次は、
引数を取り、値を返すミューテーションを呼び出す例です。

```swift
let isColorAdded: Bool = try await convex.mutation("colors:put", with: ["color": newColor])
```

### エラー処理 \{#handling-errors\}

`mutation` の呼び出し中にエラーが発生すると、例外がスローされます。通常は
[`ConvexError`](/functions/error-handling/application-errors.mdx) と `ServerError` を
catch して、アプリケーションに適した方法で処理することが多いでしょう。

次に、すでにその色が存在していた場合にエラーメッセージ付きで `ConvexError` を
スローするような `colors:put` からのエラーを処理する簡単な例を示します。

```swift
do {
  try await convex.mutation("colors:put", with: ["color": newColor])
} catch ClientError.ConvexError(let data) {
  errorMessage = try! JSONDecoder().decode(String.self, from: Data(data.utf8))
  colorNotAdded = true
}
```

詳細については、[エラー処理](/functions/error-handling/) に関するドキュメントを参照してください。

## サードパーティ API の呼び出し \{#calling-third-party-apis\}

`ConvexClient` の `action` メソッドを使って、バックエンドの
[action](/functions/actions.mdx) を呼び出せます。

`action` への呼び出しは、`mutation` への呼び出しと同様に、引数を受け取り、値を返し、例外をスローできます。

クライアントコードからアクションを呼び出すことは可能ですが、常にそれが最適な選択とは限りません。クライアントからアクションを呼び出す際のヒントについては、
[アクションのドキュメント](/functions/actions.mdx#calling-actions-from-clients) を参照してください。

## Auth0 を使用した認証 \{#authentication-with-auth0\}

`ConvexClient` の代わりに `ConvexClientWithAuth` を使用して、
[Auth0](https://auth0.com/) による認証を構成できます。そのためには
`convex-swift-auth0` ライブラリに加えて、Auth0 アカウントとアプリケーションの設定が必要です。

より詳しいセットアップ手順については
`convex-swift-auth0` リポジトリ内の
[README](https://github.com/get-convex/convex-swift-auth0/blob/main/README.md) を参照し、
Auth0 用に構成されている
[Workout example app](https://github.com/get-convex/ios-convex-workout)
も確認してください。全般的な情報源としては
[Convex authentication docs](/auth.mdx) も有用です。

他の類似した OpenID Connect 認証プロバイダと統合することも可能なはずです。詳細については
`convex-swift` リポジトリ内の
[`AuthProvider`](https://github.com/get-convex/convex-swift/blob/c47aea414c92db2ccf3a0fa4f9db8caf2029b032/Sources/ConvexMobile/ConvexMobile.swift#L188) プロトコル
を参照してください。

## 本番および dev デプロイメント \{#production-and-dev-deployments\}

アプリを[本番環境](/production.mdx)へ移行する準備ができたら、Xcode のビルドシステムを設定して、ビルドターゲットごとに異なる Convex デプロイメントを参照するようにできます。ビルド環境の構成は高度に専門的で、あなたやチームが独自の慣習を持っている可能性もありますが、ここではその問題への 1 つのアプローチ方法を示します。

1. プロジェクトのソース内に「Dev」と「Prod」のフォルダを作成します。
2. それぞれのフォルダ内に、次のような内容の `Env.swift` ファイルを追加します。

```swift
let deploymentUrl = "https://$DEV_OR_PROD.convex.cloud"
```

3. `Dev/Env.swift` に開発用の URL を、`Prod/Env.swift` に本番用の URL を設定します。
   `deploymentUrl` が複数回定義されていると Xcode に指摘されても気にしないでください。
4. 左側のエクスプローラー表示で、最上位のプロジェクトをクリックします。
5. **TARGETS** リストからビルドターゲットを選択します。
6. ターゲット名を変更し、末尾が「dev」になるようにします。
7. そのターゲットを右クリック／Ctrl-クリックして複製し、末尾が「prod」となる名前を付けます。
8. 「dev」ターゲットを選択した状態で、**Build Phases** タブをクリックします。
9. **Compile Sources** セクションを展開します。
10. `Prod/Env.swift` を選択し、- ボタンで削除します。
11. 同様に、「prod」ターゲットを開いて、`Dev/Env.swift` をそのソース一覧から削除します。

![Screenshot 2024-10-03 at 1.34.34 PM.png](/screenshots/swift_env_setup.png)

これで `ConvexClient` を作成する箇所で `deploymentUrl` を参照すると、ビルドするターゲットに応じて開発用または本番用の URL が自動的に使用されます。

## アプリケーションの構成 \{#structuring-your-application\}

このガイドで示している例は簡潔にすることを意図しており、アプリケーション全体をどのように構成するかについての指針は提供していません。

より堅牢でレイヤー化されたアプローチを取りたい場合は、`ConvexClient` とやり取りするコードを、`ObservableObject` に準拠したクラスとして切り出してください。次に、そのオブジェクトを `View` から `@StateObject` として監視し、変更があったときに再描画されるようにします。

たとえば、上記の `colors:get` の例を `ViewModel: ObservableObject` クラスに落とし込むと、`View` はもはやデータ取得に直接関与せず、`colors` のリストが `ViewModel` によって提供されることだけを把握している状態になります。

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

ニーズやアプリの規模によっては、https://github.com/nalexn/clean-architecture-swiftui のような例で示されているように、さらに形式的でしっかりした構造を持たせるのが理にかなっている場合もあります。

## 内部的な仕組み \{#under-the-hood\}

Swift Convex ライブラリは、公式の [Convex Rust client](/client/rust.md) を基盤として実装されています。Convex バックエンドとの WebSocket 接続の維持と、Convex プロトコル全体の実装を担います。

`ConvexClient` のすべてのメソッド呼び出しは、Rust 側の Tokio 非同期ランタイムを通じて処理され、アプリケーションのメインアクターから安全に呼び出すことができます。