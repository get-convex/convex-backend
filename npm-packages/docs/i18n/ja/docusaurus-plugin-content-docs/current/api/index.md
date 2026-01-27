---
id: "index"
title: "convex"
custom_edit_url: null
---

# Convex \{#convex\}

Convex 向けの TypeScript バックエンド SDK、クライアントライブラリ、および CLI です。

Convex は、プロダクトを構築するために必要なものがすべて揃ったバックエンドアプリケーションプラットフォームです。

[docs.convex.dev](https://docs.convex.dev) から始めましょう。

または [Convex demos](https://github.com/get-convex/convex-demos) を参照してください。

このリポジトリでは、Convex の TypeScript/JavaScript クライアント、Convex CLI、あるいは Convex プラットフォーム全般について、ディスカッションや issue を自由に作成してください。

また、機能リクエスト、プロダクトに関するフィードバック、一般的な質問などがあれば、[Convex Discord Community](https://convex.dev/community) で共有してください。

# 構成 \{#structure\}

このパッケージには、Convex 上でアプリを構築するための複数のエントリポイントが含まれています:

* [`convex/server`](https://docs.convex.dev/api/modules/server): Convex バックエンド関数やデータベーススキーマの定義などを行うための SDK。
* [`convex/react`](https://docs.convex.dev/api/modules/react): React アプリケーションに Convex を統合するためのフックと `ConvexReactClient`。
* [`convex/browser`](https://docs.convex.dev/api/modules/browser): Convex をその他のブラウザ環境で使用するための `ConvexHttpClient`。
* [`convex/values`](https://docs.convex.dev/api/modules/values): Convex に保存された値を扱うためのユーティリティ。
* [`convex/react-auth0`](https://docs.convex.dev/api/modules/react_auth0): Auth0 でユーザーを認証するための React コンポーネント。
* [`convex/react-clerk`](https://docs.convex.dev/api/modules/react_clerk): Clerk でユーザーを認証するための React コンポーネント。
* [`convex/nextjs`](https://docs.convex.dev/api/modules/nextjs): Next.js やその他の React フレームワークで使用できる、SSR 向けのサーバーサイドヘルパー。

このパッケージには、Convex プロジェクトを管理するためのコマンドラインインターフェース (CLI) である [`convex`](https://docs.convex.dev/using/cli) も含まれます。