---
title: "CLI"
sidebar_position: 110
slug: "cli"
description: "Convex プロジェクトと関数を管理するためのコマンドラインインターフェース"
---

Convex のコマンドラインインターフェース (CLI) は、Convex プロジェクトや
Convex 関数を管理するためのツールです。

CLI をインストールするには、以下を実行します:

```sh
npm install convex
```

全コマンドの一覧は、次のコマンドで確認できます：

```sh
npx convex
```

## 設定 \{#configure\}

### 新しいプロジェクトを作成する \{#create-a-new-project\}

初めて実行する場合

```sh
npx convex dev
```

このデバイスでログインし、新しい Convex プロジェクトを作成するよう求められます。その後、次のものが作成されます。

1. `convex/` ディレクトリ: クエリとミューテーション関数を配置する場所です。
2. `CONVEX_DEPLOYMENT` 変数を含む `.env.local`: これは Convex プロジェクトの主要な
   設定です。開発用デプロイメントの名前になります。

### プロジェクト設定を再構成する \{#recreate-project-configuration\}

次を実行します

```sh
npx convex dev
```

新規または既存のプロジェクトを構成するには、`CONVEX_DEPLOYMENT` が設定されていないプロジェクトディレクトリで実行します。

### ログアウト \{#log-out\}

```sh
npx convex logout
```

デバイスから既存の Convex の認証情報を削除し、その後に実行する `npx convex dev` などのコマンドで別の Convex アカウントを使用できるようにします。

## 開発 \{#develop\}

### Convex dev サーバーを起動する \{#run-the-convex-dev-server\}

```sh
npx convex dev
```

ローカルのファイルシステムを監視します。[関数](/functions.mdx) や
[スキーマ](/database/schemas.mdx) を変更すると、新しいバージョンが開発用
デプロイメントにプッシュされ、`convex/_generated` 内の
[生成された型](/generated-api/) が更新されます。デフォルトでは、開発用
デプロイメントからのログがターミナルに表示されます。

開発用に
[Convex デプロイメントをローカルで実行する](/cli/local-deployments-for-dev.mdx)
こともできます。

### ダッシュボードを開く \{#open-the-dashboard\}

```sh
npx convex dashboard
```

[Convex ダッシュボード](./dashboard) を開いてください。

### ドキュメントを開く \{#open-the-docs\}

```sh
npx convex docs
```

このドキュメントに戻る

### Convex 関数を実行する \{#run-convex-functions\}

```sh
npx convex run <functionName> [args]
```

開発デプロイメント環境で公開または内部の Convex クエリ、ミューテーション、アクションを実行します。

引数は JSON オブジェクトで指定します。

```sh
npx convex run messages:send '{"body": "hello", "author": "me"}'
```

クエリの結果をリアルタイムに更新するには `--watch` を付けます。関数を実行する前にローカルコードをデプロイメントにプッシュするには `--push` を付けます。

`--prod` を使用して、プロジェクトの本番デプロイメントで関数を実行します。

### デプロイメントのログをテイルする \{#tail-deployment-logs\}

dev デプロイメントのログをコンソールに出力する方法を選択できます:

```sh
# Show all logs continuously
npx convex dev --tail-logs always

# Pause logs during deploys to see sync issues (default)
npx convex dev

# 開発中はログを表示しない
npx convex dev --tail-logs disable

# Tail logs without deploying
npx convex logs
```

本番デプロイメントのログをリアルタイムで表示したい場合は、代わりに `npx convex logs` に `--prod` を付けて実行してください。

### ファイルからデータをインポート \{#import-data-from-a-file\}

```sh
npx convex import --table <tableName> <path>
npx convex import <path>.zip
```

説明とユースケースについては、次を参照してください：
[データのインポート](/database/import-export/import.mdx)。

### データをファイルにエクスポートする \{#export-data-to-a-file\}

```sh
npx convex export --path <directoryPath>
npx convex export --path <filePath>.zip
npx convex export --include-file-storage --path <path>
```

説明とユースケースは次を参照してください:
[データのエクスポート](/database/import-export/export.mdx)。

### テーブルのデータを表示する \{#display-data-from-tables\}

```sh
npx convex data  # テーブルを一覧表示
npx convex data <table>
```

コマンドラインで
[ダッシュボードのデータページ](/dashboard/deployments/data.md) のシンプルなビューを表示します。

このコマンドでは、表示されるデータを変更するために `--limit` フラグと `--order` フラグを指定できます。より複雑なフィルターが必要な場合は、ダッシュボードのデータページを使用するか、
[クエリ](/database/reading-data/reading-data.mdx) を記述してください。

`npx convex data <table>` コマンドは、`_storage` などの
[システムテーブル](/database/advanced/system-tables.mdx) に加えて、ユーザー定義のテーブルにも使用できます。

### 環境変数の読み書き \{#read-and-write-environment-variables\}

```sh
npx convex env list
npx convex env get <name>
npx convex env set <name> <value>
npx convex env remove <name>
```

デプロイメントの環境変数を表示および更新できます。これらは通常、ダッシュボードの
[environment variables settings page](/dashboard/deployments/settings.md#environment-variables)
から管理します。

## デプロイ \{#deploy\}

### Convex 関数を本番環境にデプロイする \{#deploy-convex-functions-to-production\}

```sh
npx convex deploy
```

プッシュ先となる対象のデプロイメントは、次のように決まります。

1. `CONVEX_DEPLOY_KEY` 環境変数が設定されている場合（CI で一般的）、そのキーに関連付けられているデプロイメントが対象になります。
2. `CONVEX_DEPLOYMENT` 環境変数が設定されている場合（ローカル開発時に一般的）、対象となるのは、`CONVEX_DEPLOYMENT` で指定されたデプロイメントが属しているプロジェクトの本番デプロイメントです。これにより、dev デプロイメントに対して開発しながら、prod デプロイメントにデプロイできます。

このコマンドは次のことを行います：

1. `--cmd` でコマンドが指定されていれば、それを実行します。コマンド内では CONVEX&#95;URL（もしくは同様の）環境変数が利用可能です:
   ```sh
   npx convex deploy --cmd "npm run build"
   ```
   `--cmd-url-env-var-name` を使って、URL 用の環境変数名をカスタマイズできます:
   ```sh
   npx convex deploy --cmd 'npm run build' --cmd-url-env-var-name CUSTOM_CONVEX_URL
   ```
2. Convex 関数の型チェックを行います。
3. `convex/_generated` ディレクトリ内の[生成コード](/generated-api/)を再生成します。
4. Convex 関数とその依存関係をバンドルします。
5. 関数、[インデックス](/database/reading-data/indexes/indexes.md)、および[スキーマ](/database/schemas.mdx)を本番環境にプッシュします。

このコマンドが成功すると、新しい関数はすぐに利用可能になります。

### Convex 関数を[プレビュー用デプロイメント](/production/hosting/preview-deployments.mdx)にデプロイする \{#deploy-convex-functions-to-a-preview-deployment\}

```sh
npx convex deploy
```

`CONVEX_DEPLOY_KEY` 環境変数に
[プレビューデプロイキー](/cli/deploy-key-types.mdx#deploying-to-preview-deployments)
を設定して実行すると、このコマンドは次のことを行います:

1. 新しい Convex デプロイメントを作成します。`npx convex deploy` は Vercel、Netlify、GitHub、GitLab の各環境では Git ブランチ名を自動的に判別します。あるいは、`--preview-create` オプションを使って、新しく作成されるデプロイメントに紐づく名前をカスタマイズできます。
   ```
   npx convex deploy --preview-create my-branch-name
   ```

2. `--cmd` で指定されたコマンドを実行します。コマンドからは CONVEX&#95;URL（または同様の）環境変数を利用できます:

   ```sh
   npx convex deploy --cmd "npm run build"
   ```

   `--cmd-url-env-var-name` で URL 環境変数名をカスタマイズできます:

   ```sh
   npx convex deploy --cmd 'npm run build' --cmd-url-env-var-name CUSTOM_CONVEX_URL
   ```

3. Convex 関数の型チェックを行います。

4. `convex/_generated` ディレクトリ内の
   [生成コード](/generated-api/) を再生成します。

5. Convex 関数とその依存関係をバンドルします。

6. 関数、
   [インデックス](/database/reading-data/indexes/indexes.md)、
   および [スキーマ](/database/schemas.mdx) をデプロイメントにプッシュします。

7. `--preview-run` で指定された関数を実行します（`npx convex dev` の
   `--run` オプションと同様です）。

   ```sh
   npx convex deploy --preview-run myFunction
   ```

フロントエンドとバックエンドのプレビューをまとめてセットアップする方法については、
[Vercel](/production/hosting/vercel.mdx#preview-deployments) または
[Netlify](/production/hosting/netlify.mdx#deploy-previews) のホスティングガイドを参照してください。

### 生成されたコードを更新する \{#update-generated-code\}

```sh
npx convex codegen
```

`convex/_generated` ディレクトリ内の [生成コード](/generated-api/)には、TypeScript の型チェックに必要な型が含まれています。このコードは `npx convex dev` の実行中に必要に応じて生成され、このコードはリポジトリにコミットする必要があります（これがないとコードは型チェックを通りません！）。

コードを再生成することが有用なまれなケース（たとえば、CI で正しいコードがコミットされていることを確認する場合など）では、このコマンドを使用できます。

コード生成では、Convex の JavaScript ランタイム内で設定ファイルを評価するために Convex デプロイメントと通信が発生する場合があります。これはデプロイメント上で動作しているコードを変更するものではありません。
