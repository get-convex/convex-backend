---
title: "データ"
slug: "data"
sidebar_position: 5
description:
  "ダッシュボードでデータベースのテーブルとドキュメントを表示、編集、管理する"
---

![データ ダッシュボードページ](/screenshots/data.png)

[データページ](https://dashboard.convex.dev/deployment/data)では、
すべてのテーブルとドキュメントを表示・管理できます。

ページの左側にはテーブルの一覧があります。テーブルをクリックすると、
そのテーブル内のドキュメントを作成、表示、更新、削除できます。

各テーブルの列ヘッダーはドラッグ＆ドロップして、データの表示順を変更できます。

データページの読み取り専用ビューは
[コマンドラインインターフェイス (CLI)](/cli.md#display-data-from-tables) からも利用できます。

```sh
npx convex data [table]
```

## ドキュメントのフィルタリング \{#filtering-documents\}

ページ上部の「Filter」ボタンをクリックすると、データページ上のドキュメントをフィルタリングできます。

![Data filters](/screenshots/data_filters.png)

ドキュメント内のすべてのフィールドは、Convex のクエリ構文でサポートされている演算を使ってフィルタリングできます。[Equality](/database/reading-data/filters.mdx#equality-conditions) と
[comparisons](/database/reading-data/filters.mdx#comparisons) は、Convex クライアントを使ったクエリと同様に、ダッシュボードでフィルタリングする場合も同じルールが適用されます。フィールドの型に基づいてフィルタリングすることもできます。

フィルタ条件を追加するには、既存のフィルタの横にある `+` をクリックします。条件を複数追加した場合は、それらは `and` 演算で評価されます。

各フィルタでは、フィルタリング対象のフィールド、演算、比較値を選択する必要があります。3つ目の入力ボックス（値の選択）では、有効な Convex の値を入力できます。たとえば `"a string"`、`123`、あるいは `{ a: { b: 2 } }` のような複雑なオブジェクトも指定できます。

<Admonition type="note">
  `_creationTime` でフィルタリングする場合は、通常の JavaScript 構文用の入力ボックスではなく、日付ピッカーが表示されます。`_creationTime` に対する比較はナノ秒精度で行われるため、ある時刻にぴったり一致するようにフィルタリングしたい場合は、`creationTime >= $time` と
  `creationTime <= $time + 1 minute` の2つのフィルタ条件を追加してみてください。
</Admonition>

## カスタムクエリの作成 \{#writing-custom-queries\}

ダッシュボード上で直接[クエリ](/database/reading-data/reading-data.mdx)を書くことができます。これにより、ソート、結合、グルーピング、集計を含む任意のフィルタリングや変換をデータに対して実行できます。

データページ上部の `⋮` オーバーフローメニューから「Custom query」オプションをクリックします。

<img src="/screenshots/data_custom_query.png" alt="Custom query ボタン" width={250} />

これにより、
[running your deployed functions](/dashboard/deployments/functions.md#running-functions)
と同じ UI が開きますが、「Custom test query」オプションが選択された状態になっています。このオプションにより、クエリのソースコードを編集できます。このソースコードはデプロイメントに送信され、「Run Custom Query」ボタンをクリックしたときに実行されます。

![カスタムテストクエリの実行](/screenshots/data_custom_query_runner.png)

データページ上にいない場合でも、すべてのデプロイメントページの右下に常に表示されている *fn* ボタンからこの UI を開くことができます。関数ランナーを開くキーボードショートカットは Ctrl + `（バッククオート）です。

## テーブルの作成 \{#creating-tables\}

ダッシュボードで「Create Table」ボタンをクリックし、新しいテーブル名を入力すると、テーブルを作成できます。

## ドキュメントの作成 \{#creating-documents\}

データテーブルのツールバーにある「Add Documents」ボタンを使って、個別のドキュメントをテーブルに追加できます。

「Add Documents」をクリックするとサイドパネルが開き、JavaScript 構文を使ってテーブルに新しいドキュメントを追加できます。複数のドキュメントを一度に追加するには、エディタ内の配列に新しいオブジェクトを要素として追加してください。

![Add document](/screenshots/data_add_document.png)

## クイックアクション（コンテキストメニュー） \{#quick-actions-context-menu\}

ドキュメントまたは値を右クリックすると、値のコピー、選択した値による絞り込み、ドキュメントの削除などのクイックアクションを含むコンテキストメニューが表示されます。

![クイックアクションのコンテキストメニュー](/screenshots/data_context_menu.png)

## セルの編集 \{#editing-a-cell\}

セルの値を編集するには、データテーブル内のセルをダブルクリックするか、セルが選択されている状態で Enter キーを押します。矢印キーを使って、選択中のセルを移動できます。

値はセル内で直接編集し、Enter を押して保存できます。

<Admonition type="note">
  ここでは、値の型そのものも編集できます。ただし、あなたの[スキーマ](/database/schemas.mdx)を満たしている必要があります。文字列をオブジェクトに置き換えて試してみてください！
</Admonition>

![インライン値エディター](/screenshots/data_edit_inline.png)

## ドキュメントを編集する \{#editing-a-document\}

ドキュメント内の複数のフィールドを同時に編集するには、ドキュメントにカーソルを合わせて右クリックし、コンテキストメニューを開きます。そこで「Edit Document」をクリックします。

![ドキュメント全体を編集](/screenshots/data_edit_document.png)

## 他のドキュメントへの参照を追加する \{#adding-references-to-other-documents\}

別のドキュメントを参照するには、参照したいドキュメントの文字列 ID を指定します。

セルをクリックしてから Ctrl/Cmd + C を押すと、その ID をコピーできます。

## ドキュメントの一括編集 \{#bulk-editing-documents\}

複数のドキュメントやすべてのドキュメントを一括で編集できます。すべてのドキュメントを選択するには、テーブルのヘッダー行にあるチェックボックスをクリックします。個別のドキュメントを選択するには、左端のセルにマウスカーソルを合わせ、表示されるチェックボックスをクリックします。隣接する複数のドキュメントを一度に選択するには、チェックボックスをクリックする際に Shift キーを押しながら操作します。

少なくとも 1 件のドキュメントが選択されていると、テーブルのツールバーに「(Bulk) Edit Document(s)」ボタンが表示されます。このボタンをクリックすると、右側にエディターが表示されます。

![Bulk edit documents](/screenshots/data_bulk_edit.png)

## ドキュメントの削除 \{#deleting-documents\}

少なくとも 1 つ以上のドキュメントが選択されていると（上記参照）、テーブルツールバーに「Delete Document(s)」ボタンが表示されます。ボタンをクリックするとドキュメントが削除されます。本番デプロイメントでデータを編集している場合は、ドキュメントが削除される前に確認ダイアログが表示されます。

## テーブルの内容をクリアする \{#clear-a-table\}

データページ上部の `⋮` オーバーフローメニューをクリックし、「Clear Table」をクリックすると、すべてのドキュメントを削除することもできます。このアクションはテーブル自体は削除せず、そのテーブル内のすべてのドキュメントのみを削除します。

本番環境では、Convex ダッシュボード上で、削除前にテーブル名の入力が求められます。

## テーブルの削除 \{#delete-a-table\}

<Admonition type="caution" title="これは取り消しできないアクションです">
  テーブルの削除は元に戻せません。本番環境では、削除前に Convex
  ダッシュボード上でテーブル名の入力が求められます。
</Admonition>

「Delete table」ボタンは、データページ上部の `⋮` オーバーフローメニューを
クリックすると表示されます。このアクションにより、このテーブル内のすべての
ドキュメントが削除され、このテーブル自体もテーブル一覧から削除されます。
このテーブルにインデックスが定義されていた場合、それらを再作成するには
Convex 関数を再デプロイする必要があります（本番環境または開発環境に応じて、
それぞれ `npx convex deploy` または `npx convex dev` を実行してください）。

## スキーマの生成 \{#generating-a-schema\}

ページ左下に「Generate Schema」ボタンがあり、これをクリックすると、
このテーブル内のすべてのドキュメントを対象とした[スキーマ](/database/schemas.mdx)を
Convex が生成します。

![Generate Schema button](/screenshots/data_generate_schema.png)

## テーブルのスキーマを表示する \{#view-the-schema-of-a-table\}

データページの上部にある `⋮` オーバーフローメニューをクリックすると、&quot;Schema&quot; ボタンが見つかります。

このボタンをクリックすると、選択したテーブルに関連付けられた保存済みおよび生成済みの
[スキーマ](/database/schemas.mdx) を表示するパネルが開きます。

## テーブルのインデックスを表示する \{#view-the-indexes-of-a-table\}

データページ上部の `⋮` オーバーフローメニューをクリックすると、「Indexes」ボタンが表示されます。

このボタンをクリックすると、選択したテーブルに関連付けられている
[indexes](/database/reading-data/indexes/indexes.md)
が表示されるパネルが開きます。

バックフィル処理が完了していないインデックスには、その名前の横にローディングスピナーが表示されます。