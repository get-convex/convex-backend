---
title: "Android Kotlin"
sidebar_label: "Android Kotlin"
sidebar_position: 600
description:
  "Convex を利用するモバイルアプリケーション向けの Android Kotlin クライアントライブラリ"
---

Convex Android クライアントライブラリを使用すると、Android アプリケーションから
Convex バックエンドとやり取りできます。これによりフロントエンドコードから次のことが行えます:

1. [クエリ](/functions/query-functions.mdx)、[ミューテーション](/functions/mutation-functions.mdx)、[アクション](/functions/actions.mdx) を呼び出す
2. [Auth0](/auth/auth0.mdx) を使ってユーザーを認証する

このライブラリはオープンソースで、
[GitHub から入手できます](https://github.com/get-convex/convex-mobile/tree/main/android)。

まずは [Android クイックスタート](/quickstart/android.mdx) に従ってセットアップを始めましょう。

## インストール \{#installation\}

アプリの `build.gradle[.kts]` ファイルに、次の変更を行ってください。

```kotlin
plugins {
    // ... existing plugins
    kotlin("plugin.serialization") version "1.9.0"
}

dependencies {
    // ... 既存の依存関係
    implementation("dev.convex:android-convexmobile:0.4.1@aar") {
        isTransitive = true
    }
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.3")
}
```

その後、Gradle を同期してこれらの変更を反映させてください。これでアプリから
Convex for Android ライブラリと、コードと Convex のバックエンド間の通信に使用される
Kotlin の JSON シリアライゼーションを利用できるようになります。

## バックエンドへの接続 \{#connecting-to-a-backend\}

`ConvexClient` は、アプリケーションと Convex バックエンドとの間の接続を確立し、維持するために使用されます。まずは、バックエンドのデプロイメントURL を指定してクライアントインスタンスを作成します。

```kotlin
package com.example.convexapp

import dev.convex.android.ConvexClient

val convex = ConvexClient("https://あなたのドメイン.convex.cloud")
```

アプリケーションプロセスの存続期間全体を通して、`ConvexClient` のインスタンスは 1 つだけ作成して使用するようにしてください。カスタム Android
[`Application`](https://developer.android.com/reference/android/app/Application)
サブクラスを作成し、そこで初期化しておくと便利です。

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

これで、Jetpack Compose の `@Composable` 関数から次のようにクライアントにアクセスできます：

```kotlin
val convex = (application as MyApplication).convex
```

## データの取得 \{#fetching-data\}

Android 向け Convex では Convex の
[reactor](https://docs.convex.dev/tutorial/reactor) にアクセスでき、これにより
クエリ結果へのリアルタイムな *サブスクリプション* が可能になります。`ConvexClient`
の `subscribe` メソッドでクエリを購読すると、`Flow` が返されます。
そのクエリの基盤となるデータが変化すると、それに応じて `Flow` の中身も時間とともに更新されていきます。

`ConvexClient` のすべてのメソッドは suspend 関数であり、
`CoroutineScope` か別の `suspend` 関数から呼び出す必要があります。
`@Composable` から文字列リストを返すクエリの結果をシンプルに扱うには、
リストを保持するミュータブルな state と `LaunchedEffect` を組み合わせて使う方法が考えられます。

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

バックエンドの `"workouts:get"` クエリの元になるデータが変更されるたびに、新しい
`Result<List<String>>` が `Flow` に流れてきて、`workouts` リストが新しいデータで
リフレッシュされます。`workouts` を使用しているあらゆる UI はそのたびに再ビルドされ、
完全にリアクティブな UI が実現します。

注記: サブスクリプションのロジックは、
[Android のアーキテクチャパターン](https://developer.android.com/topic/architecture/data-layer)
で説明されているように Repository でラップして実装する方法を好む場合もあります。

### クエリ引数 \{#query-arguments\}

`subscribe` に引数を渡すと、その引数は対応するバックエンド側の `query` 関数に渡されます。引数の型は
`Map<String, Any?>` 型です。マップ内の値はプリミティブな値か、または他のマップやリストでなければなりません。

```kotlin
val favoriteColors = mapOf("favoriteColors" to listOf("blue", "red"))
client.subscribe<List<String>>("users:list", args = favoriteColors)
```

`favoriteColors` 引数を受け取るバックエンドのクエリがあると仮定すると、その値を受け取り、
クエリ関数内でロジックを実行するために利用できます。

<Admonition type="tip">
  シリアライズ可能な [Kotlin Data classes](/client/android/data-types.md#custom-data-types)
  を使用して、Convex オブジェクトを Kotlin のモデルクラスに自動変換します。
</Admonition>

<Admonition type="caution">
  * Kotlin と Convex 間で
    [数値を送受信する](/client/android/data-types.md#numerical-types)
    際には、注意すべき重要なポイントがいくつかあります。
  * `_` は Kotlin でプライベートフィールドを表すために使われます。Kotlin で警告なしに
    `_creationTime` と `_id` の Convex フィールドをそのまま使いたい場合は、
    [Kotlin でフィールド名を変換する](/client/android/data-types.md#field-name-conversion)
    必要があります。
  * 利用するバックエンド関数によっては、
    [Kotlin の予約語](/client/android/data-types.md#field-name-conversion)
    を扱う必要があるかもしれません。
</Admonition>

### サブスクリプションの有効期間 \{#subscription-lifetime\}

`subscribe` から返される `Flow` は、その結果を受け取る側が存在する限り保持されます。サブスクリプションを持つ `@Composable` や `ViewModel` がスコープ外になると、Convex への背後で動作しているクエリのサブスクリプションはキャンセルされます。

## データの編集 \{#editing-data\}

`ConvexClient` の `mutation` メソッドを使って、バックエンドの
[ミューテーション](https://docs.convex.dev/functions/mutation-functions) を実行できます。

これを別の `suspend` 関数内か、`CoroutineScope` で使用する必要があります。
ミューテーションは値を返しても返さなくてもかまいません。レスポンスの型を期待する場合は、
呼び出しシグネチャでその型を指定します。

ミューテーションはクエリと同様に引数も受け取れます。引数を取るミューテーションから
型を返す例を次に示します。

```kotlin
val recordsDeleted = convex.mutation<@ConvexNum Int>(
  "messages:cleanup",
  args = mapOf("keepLatest" to 100)
)
```

`mutation` の呼び出し時にエラーが発生すると、例外がスローされます。
通常は
[`ConvexError`](https://docs.convex.dev/functions/error-handling/application-errors)
と `ServerError` をキャッチし、アプリケーションにとって適切な方法で処理します。
詳細については
[エラー処理](https://docs.convex.dev/functions/error-handling/) に関するドキュメントを参照してください。

## サードパーティ API の呼び出し \{#calling-third-party-apis\}

`ConvexClient` の `action` メソッドを使って、バックエンドの
[action](https://docs.convex.dev/functions/actions) を実行できます。

`action` の呼び出しでは、`mutation` の呼び出しと同様に、引数の受け取り、戻り値の返却、例外のスローができます。

Android からアクションを呼び出すことはできますが、常にそれが最適な選択とは限りません。
クライアントからアクションを呼び出す際のヒントについては、
[クライアントからのアクション呼び出し](https://docs.convex.dev/functions/actions#calling-actions-from-clients) を参照してください。

## Auth0 を使用した認証 \{#authentication-with-auth0\}

`ConvexClient` の代わりに `ConvexClientWithAuth` を使用して、
[Auth0](https://auth0.com/) を使った認証を設定できます。そのためには
`convex-android-auth0` ライブラリに加えて、Auth0 のアカウントと
アプリケーションの設定が必要です。

より詳細なセットアップ手順については、
`convex-android-auth0` リポジトリ内の
[README](https://github.com/get-convex/convex-android-auth0/blob/main/README.md)
を参照し、Auth0 用に構成されている
[Workout example app](https://github.com/get-convex/android-convex-workout)
も確認してください。あわせて、
[Convex authentication docs](https://docs.convex.dev/auth)
も参考になります。

同様の OpenID Connect 認証プロバイダを統合することも可能です。詳しくは、
`convex-mobile` リポジトリ内の
[`AuthProvider`](https://github.com/get-convex/convex-mobile/blob/5babd583631a7ff6d739e1a2ab542039fd532548/android/convexmobile/src/main/java/dev/convex/android/ConvexClient.kt#L291)
インターフェースを参照してください。

## 本番および dev デプロイメント \{#production-and-dev-deployments\}

アプリを[本番環境](https://docs.convex.dev/production)に移行する準備ができたら、Android のビルドシステムをセットアップして、アプリケーションの異なるビルドやフレーバーが、それぞれ別の Convex デプロイメントを参照するようにできます。比較的シンプルな方法としては、ビルドターゲットやフレーバーごとに異なる値（例: デプロイメントURL）を渡すやり方があります。

以下は、リリースビルドとデバッグビルドで異なるデプロイメントURLを使う簡単な例です。

```kotlin
// In the android section of build.gradle.kts:
buildTypes {
    release {
        // ProGuard などの各種設定は省略 ...
        resValue("string", "convex_url", "YOUR_PROD.convex.cloud")
    }

    debug {
        resValue("string", "convex_url", "YOUR_DEV.convex.cloud")
    }
}
```

その後、コード内で1つのリソースだけで `ConvexClient` を構築でき、
コンパイル時に正しい値が設定されるようになります。

```kotlin
val convex = ConvexClient(context.getString(R.string.convex_url))
```

<Admonition type="tip">
  これらの URL をリポジトリにコミットしたくない場合もあるでしょう。一つの方法としては、`.gitignore` に追加して Git の管理対象外にするカスタムの `my_app.properties` ファイルを作成するパターンがあります。その後、このファイルを `build.gradle.kts` ファイル内で読み込むことができます。このパターンの使用例は
  [workout sample app](https://github.com/get-convex/android-convex-workout?tab=readme-ov-file#configuration)
  で確認できます。
</Admonition>

## アプリケーションの構成 \{#structuring-your-application\}

このガイドで示している例は簡潔さを重視しており、アプリケーション全体の構成方法についてのガイドは提供していません。

公式の
[Android application architecture](https://developer.android.com/topic/architecture/intro)
ドキュメントでは、アプリケーション構築のベストプラクティスが説明されており、Convex には、小規模なマルチスクリーンアプリケーションがどのようなものかを例示することを目的とした
[オープンソースのサンプルアプリケーション](https://github.com/get-convex/android-convex-workout/tree/main)
もあります。

一般的には、次のようにしてください。

1. Flow と
   [unidirectional data flow](https://developer.android.com/develop/ui/compose/architecture#udf)
   を積極的に採用する
2. 明確な
   [data layer](https://developer.android.com/topic/architecture/data-layer)
   を持つ（`ConvexClient` をデータソースとして使用する Repository クラスを利用する）
3. UI の状態は
   [ViewModel](https://developer.android.com/topic/architecture/recommendations#viewmodel)
   に保持する

## テスト \{#testing\}

`ConvexClient` は `open` クラスなので、ユニットテストでモック化したりフェイクとして扱うことができます。より実際のクライアントに近い形で使いたい場合は、フェイクの
`MobileConvexClientInterface` を `ConvexClient` コンストラクタに渡せます。ただし、その場合は Convex の非公開仕様の
[JSON 形式](https://github.com/get-convex/convex-mobile/blob/5babd583631a7ff6d739e1a2ab542039fd532548/android/convexmobile/src/main/java/dev/convex/android/jsonhelpers.kt#L47)
に従った JSON を用意する必要がある点に注意してください。

Android のインストルメンテーションテストでは、フル機能の `ConvexClient` を使用することもできます。テスト専用のバックエンドインスタンスをセットアップするか、ローカルで Convex サーバーを起動して、フルの統合テストを実行できます。

## 内部動作 \{#under-the-hood\}

Android 向け Convex は、公式の
[Convex Rust client](https://docs.convex.dev/client/rust) を基盤として構築されています。これが
Convex バックエンドとの WebSocket 接続の維持を行い、Convex プロトコル全体を実装します。

`ConvexClient` 上のすべてのメソッド呼び出しは、Rust 側の Tokio の非同期ランタイム経由で処理され、
アプリケーションのメインスレッドから安全に呼び出すことができます。

`ConvexClient` はまた
[Kotlin のシリアライゼーションフレームワーク](https://github.com/Kotlin/kotlinx.serialization/blob/master/docs/serialization-guide.md)
を多用しており、そのほとんどの機能をアプリケーション内で利用できます。内部的には、
`ConvexClient` は JSON の
[`ignoreUnknownKeys`](https://github.com/Kotlin/kotlinx.serialization/blob/master/docs/json.md#ignoring-unknown-keys)
および
[`allowSpecialFloatingPointValues`](https://github.com/Kotlin/kotlinx.serialization/blob/master/docs/json.md#allowing-special-floating-point-values)
の各機能を有効にしています。