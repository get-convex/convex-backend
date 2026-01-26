---
title: "関数"
slug: "functions"
sidebar_position: 10
description:
  "メトリクスとパフォーマンスデータを用いて Convex 関数を実行、テスト、監視する"
---

![Functions ダッシュボードビュー](/screenshots/functions.png)

[Functions ページ](https://dashboard.convex.dev/deployment/functions)には、
現在デプロイされているすべての Convex 関数が表示されます。

開発用デプロイメントでは、
[`npx convex dev`](/cli.md#run-the-convex-dev-server) によってこれらは継続的に更新されます。
本番デプロイメントでは、関数は
[`npx convex deploy`](/cli.md#deploy-convex-functions-to-production) で登録されます。

## 関数の実行 \{#running-functions\}

ダッシュボードで Convex 関数を実行するには、ページ左側の一覧から関数を選択し、その関数名の横に表示される「Run Function」ボタンをクリックします。

関数ページ以外にいる場合でも、すべてのデプロイメントページの右下に表示されている常駐の *fn* ボタンからこの UI を開くことができます。関数ランナーを開くキーボードショートカットは Ctrl + `（バッククオート）です。

このビューでは、関数の引数を入力して実行できます。

クエリの結果は、関数の引数を変更したり、データが変更されたりすると自動的に更新されます。

ミューテーションとアクションの結果は、「Run」ボタンをクリックすると表示されます。

これらの結果には、関数から出力されたログと返された値が表示される点に注意してください。
関数を実行したときに何が変化したかを確認するには、
[データページ](/dashboard/deployments/data.md) を参照してください。

![関数の実行](/screenshots/run_function.png)

また、
[カスタムクエリ関数を書く](/dashboard/deployments/data.md#writing-custom-queries)
セクションで「Custom test query」オプションを選択することで、デプロイ済み関数ではなく独自のクエリ関数を記述して実行することもできます。

### ページネーション対応関数へのクエリ実行 \{#querying-a-paginated-function\}

ダッシュボードでページネーション対応関数にクエリを実行する場合、UI は
引数に [`PaginationOptions`](/api/interfaces/server.PaginationOptions) を含めることを想定します。つまり、
`numItems` フィールドと、必要に応じて `cursor` フィールドを含むオブジェクトです。
この引数名は、クエリ関数内で定義した引数名と同じである必要があります。

* `numItems` には、1 ページに含めるアイテム数を指定します
* `cursor` は、ページネーションを開始する際は空のままにしておいて問題ありません。結果を受け取ったら、
  次のページに進むために、その結果の `continueCursor` フィールドを `cursor` に設定できます。

### ユーザーアイデンティティを仮定する \{#assuming-a-user-identity\}

<Admonition type="tip">
  Convex のダッシュボードでユーザーアイデンティティを仮定しても、
  実際のユーザーアイデンティティへアクセスできるわけではありません。
  この概念は、関数内でユーザーアイデンティティを「モックする」ものと考えることができます。
</Admonition>

認証機能付きのアプリケーションを構築している場合、認証済みユーザーとして
Convex 関数を実行したくなることがあります。

その場合は、「Act as a user」チェックボックスをオンにします。

その後、表示される入力欄に値を入力して、ユーザーアイデンティティオブジェクトを
設定できます。

![ユーザーとして動作する](/screenshots/acting_as_a_user.png)

指定できるユーザー属性は次のとおりです。

| Attribute           | Type                                     |
| ------------------- | ---------------------------------------- |
| subject*           | string                                   |
| issuer*            | string                                   |
| name                | string                                   |
| givenName           | string                                   |
| familyName          | string                                   |
| nickname            | string                                   |
| preferredUsername   | string                                   |
| profileUrl          | string                                   |
| email               | string                                   |
| emailVerified       | boolean                                  |
| gender              | string                                   |
| birthday            | string                                   |
| timezone            | string                                   |
| language            | string                                   |
| phoneNumber         | string                                   |
| phoneNumberVerified | boolean                                  |
| address             | string                                   |
| updatedAt           | string (RFC 3339 形式の日付文字列)        |
| customClaims        | object                                   |

*これらの属性は必須です。

## メトリクス \{#metrics\}

各関数ごとに 4 つの基本的なグラフがあります。チーム全体の利用状況に関するメトリクスについては、
[チーム設定](/dashboard/teams.md#usage) を参照してください。

### 呼び出し回数 \{#invocations\}

このグラフは、関数が 1 分あたりに呼び出された回数を表示します。
アプリの利用が増えるにつれて、このグラフも右肩上がりに推移していくはずです。

### エラー \{#errors\}

関数の実行中に発生した例外のプロットです。何が起きているのか知りたい場合は、この後で詳しく説明するログのページを確認してください。

### キャッシュヒット率 \{#cache-hit-rate\}

<Admonition type="tip">
  キャッシュヒット率はクエリ関数にのみ適用されます
</Admonition>

この関数が再実行されるのではなく、キャッシュされた値がそのまま再利用されている頻度を示す割合です。
キャッシュヒット率が高いほど、アプリケーションはより効率的に動作し、レスポンス時間も短くなります。

### 実行時間 \{#execution-time\}

この関数の実行にかかっている時間をミリ秒単位で示します。

このチャートには p50、p90、p95、p99 の 4 本の線が描画されています。
それぞれの線は、時間経過に伴うリクエストの分布における、そのパーセンタイルでの
応答時間を表しています。たとえば、p99 の線で示される時間より長く実行に
かかったリクエストは全体の 1% だけという意味です。一般的に、これらの
*テイルレイテンシ* を注視することは、アプリケーションがデータサービスに
素早くアクセスできているかを確認するうえで有効です。

実行時間とキャッシュヒット率の関係も考慮してください。一般的に、
キャッシュヒットは 1ms を大きく下回る時間で完了するため、キャッシュヒット率が高いほど
応答時間は短くなります。

いずれかのチャートをクリックすると、より大きく詳細なビューが表示され、
調査したい時間範囲をカスタマイズできます。