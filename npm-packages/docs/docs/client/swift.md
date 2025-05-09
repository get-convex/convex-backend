---
title: "iOS & macOS Swift"
sidebar_label: "Swift"
sidebar_position: 700
---

The Convex Swift client library enables your iOS or macOS application to
interact with your Convex backend. It allows your frontend code to:

1. Call
   your [queries](/functions/query-functions.mdx), [mutations](/functions/mutation-functions.mdx) and [actions](/functions/actions.mdx)
2. Authenticate users using [Auth0](/auth/auth0.mdx)

The library is open source
and [available on GitHub](https://github.com/get-convex/convex-swift).

Follow the [Swift Quickstart](/quickstart/swift.mdx) to get started.

## Installation

For an iOS or macOS project in Xcode, you’ll need to perform the following steps
to add a dependency on the `ConvexMobile` library.

1. Click on the top-level app container in the project navigator on the left
2. Click on the app name under the PROJECT heading
3. Click the _Package Dependencies_ tab
4. Click the + button

   ![Screenshot 2024-10-02 at 2.33.43 PM.png](/screenshots/swift_qs_step_2.png)

5. Paste
   [`https://github.com/get-convex/convex-swift`](https://github.com/get-convex/convex-swift)
   into the search box and press Enter
6. When the `convex-swift` package loads, click the Add Package button
7. In the _Package Products_ dialog, select your product name in the _Add to
   Target_ dropdown
8. Click _Add Package_

## Connecting to a backend

The `ConvexClient` is used to establish and maintain a connection between your
application and the Convex backend. First you need to create an instance of the
client by giving it your backend deployment URL:

```swift
import ConvexMobile

let convex = ConvexClient(deploymentUrl: "https://<your domain here>.convex.cloud")
```

You should create and use one instance of the `ConvexClient` for the lifetime of
your application process. You can store the client in a global constant like
shown above. An actual connection to the Convex backend won’t be initiated until
you call a method on the `ConvexClient`. After that it will maintain the
connection and re-establish it if it gets dropped.

## Fetching data

The Swift Convex library gives you access to the Convex sync engine, which
enables real-time *subscriptions* to query results. You subscribe to queries
with the `subscribe` method on `ConvexClient` which returns
a [`Publisher`](https://developer.apple.com/documentation/combine). The data
available via the `Publisher` will change over time as the underlying data
backing the query changes.

You can call methods on the `Publisher` to transform and consume the data it
provides.

A simple way to consume a query that returns a list of strings in a `View` is to
use a combination of a `@State` containing a list and the `.task` modifier with
code that loops over the query results as an `AsyncSequence`:

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

Any time the data that powers the backend `"colors:get"` query changes, a
new array of `String` values will appear in the `AsyncSequence` and the
`View`'s `colors` list gets assigned the new data. The UI will then rebuild
reactively to reflect the changed data.

### Query arguments

You can pass arguments to `subscribe` and they will be supplied to the
associated backend `query` function. The arguments must be a Dictionary keyed
with strings and the values should generally be primitive types, Arrays and
other Dictionaries.

```swift
let publisher = convex.subscribe(to: "colors:get",
                               with:["onlyFavorites": true],
                           yielding:[String].self)
```

Assuming the `colors:get` query accepts an `onlyFavorites` argument, the value
can be received and used to perform logic in the query function.

<Admonition type="tip">
Use [Decodable structs](/client/swift/data-types.md#custom-data-types)
to automatically convert Convex objects to Swift structs.
</Admonition>

<Admonition type="caution">
* There are important gotchas when
  [sending and receiving numbers](/client/swift/data-types.md#numerical-types)
  between Swift and Convex.
* Depending on your backend functions, you may need to deal with
  [reserved Swift keywords](/client/swift/data-types.md#field-name-conversion).
</Admonition>

### Subscription lifetime

The `Publisher` returned from `subscribe` will persist as long as the associated
`View` or `ObservableObject`. When either is no longer part of the UI, the
underlying query subscription to Convex will be canceled.

## Editing Data

You can use the `mutation` method on `ConvexClient` to trigger a
backend [mutation](/functions/mutation-functions.mdx).

`mutation` is an `async` method so you'll need to call it within a `Task`.
Mutations can return a value or not.

Mutations can also receive arguments, just like queries. Here's an example of
calling a mutation with arguments that returns a value:

```swift
let isColorAdded: Bool = try await convex.mutation("colors:put", with: ["color": newColor])
```

### Handling errors

If an error occurs during a call to `mutation`, it will throw. Typically you may
want to
catch [`ConvexError`](/functions/error-handling/application-errors.mdx) and `ServerError` and
handle them however is appropriate in your application.

Here’s a small example of how you might handle an error from `colors:put` if it
threw a `ConvexError` with an error message if a color already existed.

```swift
do {
  try await convex.mutation("colors:put", with: ["color": newColor])
} catch ClientError.ConvexError(let data) {
  errorMessage = try! JSONDecoder().decode(String.self, from: Data(data.utf8))
  colorNotAdded = true
}
```

See documentation on [error handling](/functions/error-handling/) for more
details.

## Calling third-party APIs

You can use the `action` method on `ConvexClient` to trigger a
backend [action](/functions/actions.mdx).

Calls to `action` can accept arguments, return values and throw exceptions just
like calls to `mutation`.

Even though you can call actions from your client code, it's not always the
right choice. See the action docs for tips
on [calling actions from clients](/functions/actions.mdx#calling-actions-from-clients).

## Authentication with Auth0

You can use `ConvexClientWithAuth` in place of `ConvexClient` to configure
authentication with [Auth0](https://auth0.com/). You'll need
the `convex-swift-auth0` library to do that, as well as an Auth0 account and
application configuration.

See
the [README](https://github.com/get-convex/convex-swift-auth0/blob/main/README.md) in
the `convex-swift-auth0` repo for more detailed setup instructions, and
the [Workout example app](https://github.com/get-convex/ios-convex-workout) which
is configured for Auth0. The overall [Convex authentication docs](/auth.mdx) are
a good resource as well.

It should also be possible to integrate other similar OpenID Connect
authentication providers. See
the [`AuthProvider`](https://github.com/get-convex/convex-swift/blob/c47aea414c92db2ccf3a0fa4f9db8caf2029b032/Sources/ConvexMobile/ConvexMobile.swift#L188) protocol
in the `convex-swift` repo for more info.

## Production and dev deployments

When you're ready to move toward [production](/production.mdx) for your app, you
can setup your Xcode build system to point different build targets to different
Convex deployments. Build environment configuration is highly specialized, and
it’s possible that you or your team have different conventions, but this is one
way to approach the problem.

1. Create “Dev” and “Prod” folders in your project sources.
2. Add an `Env.swift` file in each one with contents like:

```swift
let deploymentUrl = "https://$DEV_OR_PROD.convex.cloud"
```

3. Put your dev URL in `Dev/Env.swift` and your prod URL in `Prod/Env.swift`.
   Don’t worry if Xcode complains that `deploymentUrl` is defined multiple
   times.
4. Click on your top-level project in the explorer view on the left.
5. Select your build target from the **TARGETS** list.
6. Change the target’s name so it ends in “dev”.
7. Right/Ctrl-click it and duplicate it, giving it a name that ends in “prod”.
8. With the “dev” target selected, click the **Build Phases** tab.
9. Expand the **Compile Sources** section.
10. Select `Prod/Env.swift` and remove it with the - button.
11. Likewise, open the “prod” target and remove `Dev/Env.swift` from its
    sources.

![Screenshot 2024-10-03 at 1.34.34 PM.png](/screenshots/swift_env_setup.png)

Now you can refer to `deploymentUrl` wherever you create your `ConvexClient` and
depending on the target that you build, it will use your dev or prod URL.

## Structuring your application

The examples shown in this guide are intended to be brief, and don't provide
guidance on how to structure a whole application.

If you want a more robust and layered approach, put your code that interacts
with `ConvexClient`in a class that conforms to `ObservableObject`. Then your
`View` can observe that object as a `@StateObject` and will rebuild whenever it
changes.

For example, if we adapt the `colors:get` example from above to a
`ViewModel: ObservableObject` class, the `View` no longer plays a direct part in
fetching the data - it only knows that the list of `colors` is provided by the
`ViewModel`.

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

Depending on your needs and the scale of your app, it might make sense to give
it even more formal structure as demonstrated in something like
https://github.com/nalexn/clean-architecture-swiftui.

## Under the hood

The Swift Convex library is built on top of the
official [Convex Rust client](/client/rust.md). It handles maintaining a
WebSocket connection with the Convex backend and implements the full Convex
protocol.

All method calls on `ConvexClient` are handled via a Tokio async runtime on the
Rust side and are safe to call from the application's main actor.
