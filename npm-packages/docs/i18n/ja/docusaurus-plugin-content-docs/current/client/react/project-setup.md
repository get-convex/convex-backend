---
title: "デプロイメントURLの設定"
slug: "deployment-urls"
sidebar_label: "デプロイメントURL"
hidden: false
sidebar_position: 5
description: "Convex 上でプロジェクトを実行するための設定"
---

[バックエンドへの接続](/client/react.mdx#connecting-to-a-backend) を行うときは、
デプロイメントURLを正しく設定することが重要です。

### Convex プロジェクトを作成する \{#create-a-convex-project\}

初めて実行するときに

```sh
npx convex dev
```

プロジェクトディレクトリ内で新しい Convex プロジェクトを作成します。

新しいプロジェクトには、*production* と *development* の 2 つのデプロイメントが含まれます。
使用しているフロントエンドフレームワークやバンドラに応じて、
*development* デプロイメントの URL は `.env.local` または `.env` ファイルに保存されます。

プロジェクト内のすべてのデプロイメントの URL は、Convex の
[ダッシュボード](https://dashboard.convex.dev)の
[deployment settings](/dashboard/deployments/settings.md)ページで確認できます。

### クライアントを設定する \{#configure-the-client\}

Convex デプロイメントの URL を渡して、Convex React クライアントを作成します。
フロントエンドアプリケーション内の Convex クライアントは、通常は 1 つだけにします。

```jsx title="src/index.js"
import { ConvexProvider, ConvexReactClient } from "convex/react";

const deploymentURL = import.meta.env.VITE_CONVEX_URL;

const convex = new ConvexReactClient(deploymentURL);
```

この URL をハードコードすることもできますが、どのデプロイメントにクライアントを接続するかを環境変数で決められるようにしておくと便利です。

利用しているフロントエンドフレームワークやバンドラに応じて、クライアントコードから参照できる環境変数名を使用してください。

### 環境変数名の選び方 \{#choosing-environment-variable-names\}

フロントエンドコードで秘密の環境変数を意図せず公開してしまうことを避けるために、
多くのバンドラーではフロントエンドコードで参照される環境変数に特定のプレフィックスを付ける必要があります。

[Vite](https://vitejs.dev/guide/env-and-mode.html) では、フロントエンドコードで使用する環境変数は `VITE_` で始まる必要があるため、`VITE_CONVEX_URL` のような名前が適しています。

[Create React App](https://create-react-app.dev/docs/adding-custom-environment-variables/)
では、フロントエンドコードで使用する環境変数は `REACT_APP_` で始まる必要があるため、
上記のコードでは `REACT_APP_CONVEX_URL` を使用しています。

[Next.js](https://nextjs.org/docs/basic-features/environment-variables#exposing-environment-variables-to-the-browser)
では `NEXT_PUBLIC_` で始まる必要があるため、`NEXT_PUBLIC_CONVEX_URL` が適した名前です。

バンドラーは、これらの変数にアクセスする方法もそれぞれ異なります。
[Vite は `import.meta.env.VARIABLE_NAME` を使用します](https://vitejs.dev/guide/env-and-mode.html) が、
Next.js のような他の多くのツールは Node.js 風の
[`process.env.VARIABLE_NAME`](https://nextjs.org/docs/basic-features/environment-variables)
を使用します。

```jsx
import { ConvexProvider, ConvexReactClient } from "convex/react";

const convex = new ConvexReactClient(process.env.NEXT_PUBLIC_CONVEX_URL);
```

[`.env` ファイル](https://www.npmjs.com/package/dotenv)は、開発環境と本番環境で
異なる環境変数の値を結び付けて設定する一般的な方法です。
`npx convex dev` は、プロジェクトが使用しているバンドラを推測しつつ、
対応する `.env` ファイルにデプロイメントURLを保存します。

```shell title=".env.local"
NEXT_PUBLIC_CONVEX_URL=https://guiltless-dog-960.convex.cloud

# フロントエンドに渡される可能性のある他の環境変数の例
NEXT_PUBLIC_SENTRY_DSN=https://123abc@o123.ingest.sentry.io/1234
NEXT_PUBLIC_LAUNCHDARKLY_SDK_CLIENT_SIDE_ID=01234567890abcdef
```

バックエンド関数は、ダッシュボード上で設定した
[環境変数](/production/environment-variables.mdx) を利用できます。`.env` ファイルから
値を読み込むことはありません。
