---
title: "Convex HTTP API"
sidebar_label: "パブリック HTTP API"
description: "HTTP 経由で Convex に直接接続する"
---

import Tabs from "@theme/Tabs"; import TabItem from "@theme/TabItem";

デプロイメントを定義する公開関数は、パブリックな HTTP エンドポイントとして外部に公開されます。

## Convex 値の形式 \{#convex-value-format\}

各 HTTP API では、ドキュメントの形式を指定する `format` クエリパラメータを受け取ります。現在サポートされている値は `json` のみです。詳細は
[型のページ](/database/types#convex-values) を参照してください。簡略化のため、`json` 形式は入力としてすべての Convex データ型をサポートしているわけではなく、出力では複数のデータ型に対して重なり合う表現を使用します。将来的には、すべての Convex データ型をサポートする新しい形式を追加する予定です。

## API 認証 \{#api-authentication\}

Functions API は、`Authorization` ヘッダーにベアラートークンを指定することで、必要に応じてユーザーとして認証できます。値は `Bearer <access_key>` であり、このキーは認証プロバイダーから発行されるトークンです。Clerk との動作の詳細は、Clerk のドキュメントの
[under the hood](/auth/clerk#under-the-hood) セクションを参照してください。

ストリーミング エクスポートおよびストリーミング インポートのリクエストには、HTTP ヘッダー `Authorization` によるデプロイメント管理者権限での認可が必要です。値は `Convex <access_key>` であり、このアクセスキーは Convex ダッシュボード上の「Deploy key」から取得するもので、Convex データの読み書きに対する完全なアクセス権を付与します。

## 関数 API \{#functions-api\}

### POST `/api/query`, `/api/mutation`, `/api/action` \{#post-apiquery-apimutation-apiaction\}

これらのHTTPエンドポイントを使うと、Convex関数を呼び出して、その結果を
値として取得できます。

バックエンドのデプロイメントURLはダッシュボードの
[Settings](/dashboard/deployments/settings.md) ページで確認できます。
そのデプロイメントURLに続けて API のパスを付けたものが API URL になり、
`<CONVEX_URL>/api/query` などになります。例えば次のように呼び出せます。

<Tabs>
  <TabItem value="shell" label="Shell">
    ```
    curl https://acoustic-panther-728.convex.cloud/api/query \
       -d '{"path": "messages:list", "args": {}, "format": "json"}' \
       -H "Content-Type: application/json"
    ```
  </TabItem>

  <TabItem value="js" label="NodeJS">
    ```js
    const url = "https://acoustic-panther-728.convex.cloud/api/query";
    const request = { path: "messages:list", args: {}, format: "json" };

    const response = fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(request),
    });
    ```
  </TabItem>

  <TabItem value="py" label="Python">
    ```py
    import requests

    url = "https://acoustic-panther-728.convex.cloud/api/query"
    headers = {"accept": "application/json"}
    body = {"path": "messages:list", "args": {}, "format": "json"}

    response = requests.post(url, headers=headers, json=body)
    ```
  </TabItem>
</Tabs>

**JSON ボディパラメータ**

| Name   | Type   | Required | Description                                                                                                  |
| ------ | ------ | -------- | ------------------------------------------------------------------------------------------------------------ |
| path   | string | y        | Convex 関数へのパスを表す文字列。[こちら](/functions/query-functions#query-names) で定義されています。           |
| args   | object | y        | Convex 関数に渡す名前付き引数オブジェクト。                                                                    |
| format | string | n        | 値の出力フォーマット。有効な値: [`json`]                                                                      |

**成功時の Result JSON**

| Field Name | Type         | Description                                      |
| ---------- | ------------ | ------------------------------------------------ |
| status     | string       | &quot;success&quot;                                        |
| value      | object       | 要求されたフォーマットでの Convex 関数の結果。    |
| logLines   | list[string] | 関数の実行中に出力されたログ行。                  |

**エラー時の Result JSON**

| Field Name   | Type         | Description                                                                                           |
| ------------ | ------------ | ----------------------------------------------------------------------------------------------------- |
| status       | string       | &quot;error&quot;                                                                                               |
| errorMessage | string       | エラーメッセージ。                                                                                    |
| errorData    | object       | スローされた場合の[アプリケーションエラー](/functions/error-handling/application-errors) 内のエラーデータ。 |
| logLines     | list[string] | 関数の実行中に出力されたログ行。                                                                       |

### POST `/api/run/{functionIdentifier}` \{#post-apirunfunctionidentifier\}

この HTTP エンドポイントを使うと、リクエスト URL のパスを指定して任意の種類の Convex 関数を呼び出し、その結果を値として取得できます。function identifier は
[ここ](/functions/query-functions#query-names) で定義されている文字列形式で、`:` の代わりに `/` を使います。

バックエンドのデプロイメントURLはダッシュボードの
[Settings](/dashboard/deployments/settings.md) ページで確認でき、その後 API の URL は
`<CONVEX_URL>/api/run/{functionIdentifier}` のようになります。例えば次のようになります:

<Tabs>
  <TabItem value="shell" label="Shell">
    ```
    curl https://acoustic-panther-728.convex.cloud/api/run/messages/list \
       -d '{"args": {}, "format": "json"}' \
       -H "Content-Type: application/json"
    ```
  </TabItem>

  <TabItem value="js" label="NodeJS">
    ```js
    const url = "https://acoustic-panther-728.convex.cloud/api/run/messages/list";
    const request = { args: {}, format: "json" };

    const response = fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(request),
    });
    ```
  </TabItem>

  <TabItem value="py" label="Python">
    ```py
    import requests

    url = "https://acoustic-panther-728.convex.cloud/api/run/messages/list"
    headers = {"accept": "application/json"}
    body = {"args": {}, "format": "json"}

    response = requests.get(url, headers=headers, body=json)
    ```
  </TabItem>
</Tabs>

**JSON ボディのパラメータ**

| Name   | Type   | Required | Description                                                       |
| ------ | ------ | -------- | ----------------------------------------------------------------- |
| args   | object | y        | Convex 関数に渡す名前付き引数オブジェクト。                      |
| format | string | n        | 値の出力フォーマット。デフォルトは `json`。有効な値: [`json`] |

**成功時の Result JSON**

| Field Name | Type         | Description                                         |
| ---------- | ------------ | --------------------------------------------------- |
| status     | string       | &quot;success&quot;                                           |
| value      | object       | 要求されたフォーマットでの Convex 関数の結果。      |
| logLines   | list[string] | 関数実行中に出力されたログ行。                      |

**エラー時の Result JSON**

| Field Name   | Type         | Description                                                                                         |
| ------------ | ------------ | --------------------------------------------------------------------------------------------------- |
| status       | string       | &quot;error&quot;                                                                                             |
| errorMessage | string       | エラーメッセージ。                                                                                  |
| errorData    | object       | 例外としてスローされた場合の、[application error](/functions/error-handling/application-errors) 内のエラーデータ。 |
| logLines     | list[string] | 関数実行中に出力されたログ行。                                                                      |