---
title: "Convex データのストリーミング入出力"
sidebar_label: "ストリーミング インポート／エクスポート"
description: "Convex データのストリーミング入出力"
sidebar_position: 4
---

[Fivetran](https://www.fivetran.com) と [Airbyte](https://airbyte.com) は、
Convex のデータを他のデータベースと同期できるデータ統合プラットフォームです。

Fivetran を使用すると、Convex からいずれかの
[対応デスティネーション](https://fivetran.com/docs/destinations) へのストリーミング エクスポートが可能です。Convex
チームは、ストリーミング エクスポート用の Convex ソースコネクタを管理しています。Fivetran 経由で Convex にストリーミング インポートすることは、現時点ではサポートしていません。

Airbyte を使用すると、いずれかの
[対応ソース](https://airbyte.com/connectors?connector-type=Sources) から Convex へのストリーミング インポートと、
Convex からいずれかの
[対応デスティネーション](https://airbyte.com/connectors?connector-type=Destinations) へのストリーミング エクスポートが可能になります。
Convex チームは、ストリーミング エクスポート用の Convex ソースコネクタと、ストリーミング インポート用の Convex デスティネーションコネクタを管理しています。

<BetaAdmonition feature="Fivetran と Airbyte の連携機能" verb="are" />

## ストリーミングエクスポート \{#streaming-export\}

データをエクスポートすることは、Convex が直接サポートしていないワークロードを扱うのに役立ちます。ユースケースの例としては次のようなものがあります。

1. 分析
   * Convex は大量のデータを読み込むようなクエリには最適化されていません。そのような用途には [Databricks](https://www.databricks.com) や
     [Snowflake](https://www.snowflake.com/) のようなデータプラットフォームの方が適しています。
2. 柔軟なクエリの実行
   * Convex には強力な
     [データベースクエリ](/database/reading-data/reading-data.mdx#querying-documents)
     と組み込みの [全文検索](/search.mdx) 機能がありますが、それでも Convex 内では記述が難しいクエリが存在します。たとえば「詳細検索」ビューのように、非常に動的なソートやフィルタリングが必要な場合は、
     [ElasticSearch](https://www.elastic.co) のようなデータベースが役立ちます。
3. 機械学習のトレーニング
   * Convex は、計算量の多い機械学習アルゴリズムを実行するクエリには最適化されていません。

<ProFeatureUpsell feature="Streaming export" verb="requires" />

ストリーミングエクスポートの設定方法については
[Fivetran](https://fivetran.com/integrations/convex) または
[Airbyte](https://docs.airbyte.com/integrations/sources/convex) のドキュメントを参照してください。サポートが必要な場合や質問がある場合は
[お問い合わせ](https://convex.dev/community) ください。

## ストリーミングインポート \{#streaming-import\}

新しい技術を採用するのは、とくにデータベースが絡む場合、時間がかかり負担に感じられることがあります。ストリーミングインポートを使うと、自前でマイグレーションやデータ同期ツールを書かなくても、既存スタックと並行して Convex を導入できます。ユースケースの例は次のとおりです。

1. Convex がプロジェクトの既存バックエンドを、そのデータを使ってどのように置き換えられるかを試作する。
2. 既存のデータベースと並行して Convex を利用し、新しいプロダクトをより速く構築する。
3. 既存データセットの上にリアクティブな UI レイヤーを構築する。
4. データを Convex に移行する（[CLI](/cli.md) ツールだけでは要件を満たせない場合）。

<Admonition type="caution" title="インポートしたテーブルは読み取り専用にする">
  よくあるユースケースとしては、ソースデータベース内のテーブルを Convex に「ミラーリング」して、Convex を使って新しい機能やプロダクトを構築する、というものがあります。インポートしたテーブルについては、結果をソースデータベースに同期し戻すと危険な書き込み競合が発生しうるため、Convex 上では読み取り専用のままにしておくことを推奨します。Convex にはまだテーブルを読み取り専用であることを保証するアクセス制御機能はありませんが、コード内でインポートしたテーブルに書き込むミューテーションやアクションを定義しないようにし、ダッシュボード上でインポートしたテーブル内のドキュメントを編集しないことで回避できます。
</Admonition>

ストリーミングインポートはすべての Convex プランに含まれます。Convex destination connector のセットアップ方法は、Airbyte のドキュメントを
[こちら](https://docs.airbyte.com/integrations/destinations/convex)で確認してください。