---
title: "ログ"
slug: "logs"
sidebar_position: 40
description:
  "リアルタイムの関数ログとデプロイメントのアクティビティをダッシュボードで表示する"
---

![Logs Dashboard Page](/screenshots/logs.png)

[logs ページ](https://dashboard.convex.dev/deployment/logs)は、
デプロイメント内で発生するすべてのアクティビティをリアルタイムに表示します。

logs ページには最近の関数ログの短い履歴が表示され、新しいログは生成され次第
反映されます。より長期間のログ履歴を保存するには、
[log stream](/production/integrations/log-streams/log-streams.mdx) を設定できます。

関数アクティビティには次のものが含まれます:

* 関数実行の時刻
* 関数実行の Request ID
* 関数実行の結果（成功または失敗）
* 呼び出された関数の名前
* 関数の出力。関数によって出力されたログ行（例:
  `console.log`）や例外を含みます。
* 関数実行の所要時間（ミリ秒単位、ネットワークレイテンシーは含みません）

関数アクティビティに加えて、
設定変更を表す [deployment events](/dashboard/deployments/history.md) もここに表示されます。

任意のログをクリックすると、選択したログと同じ Request ID に紐づくすべてのログを表示するビューが開きます。これはエラーのデバッグや
関数実行のコンテキストを理解するのに役立ちます。

![Request ID Logs](/screenshots/request_logs.png)

ページ上部のコントロールを使って、テキスト、関数名、実行ステータス、およびログの重大度でログを絞り込むことができます。

### ログをフィルタリングする \{#filter-logs\}

ページ上部の &quot;Filter logs...&quot; テキストボックスを使って、ログのテキストを絞り込めます。

&quot;Functions&quot; ドロップダウンリストを使って、結果に含める関数・除外する関数を選択できます。

&quot;Filter logs&quot; と
[Convex request id](/functions/error-handling/error-handling.mdx#debugging-errors)
を組み合わせて、特定のエラーのログを見つけることもできます。
たとえばブラウザのコンソールに次のような `Error` が表示された場合:

![Browser Error](/screenshots/console_error_requestid.png)

その Request ID を Convex ダッシュボードの
[Logs](/dashboard/deployments/logs.md) ページにある
&quot;Search logs...&quot; 検索バーに貼り付けると、
その関数のログを表示できます。
このページはログの履歴を完全に保持しているわけではないため、
古いリクエストのログは見つからない可能性がある点に注意してください。

ほとんどのエラー報告サービスやログの出力先も、Request ID で検索できるはずです。

### ログの種類 \{#log-types\}

ログは種類ごとにフィルタリングすることもできます。種類には、関数の結果（成功または失敗）と重大度レベル（info、warn、debug、error）が含まれます。

すべての失敗した実行には理由が含まれており、多くの場合は JavaScript の例外です。