---
title: "プロジェクト"
slug: "projects"
sidebar_position: 10
description: "Convex プロジェクト、設定、デプロイメントの作成と管理"
---

![Project settings](/screenshots/projects.png)

プロジェクトは Convex を使用するコードベースに対応し、その中には 1 つの本番デプロイメントと、各チームメンバーごとのパーソナルデプロイメントが含まれます。

[ランディングページ](https://dashboard.convex.dev)でプロジェクトをクリックすると、
プロジェクトの詳細ページに移動します。

## プロジェクトの作成 \{#creating-a-project\}

プロジェクトはダッシュボードまたは
[CLI](/cli.md#create-a-new-project) から作成できます。ダッシュボードからプロジェクトを作成するには、
Create Project ボタンをクリックしてください。

## プロジェクト設定 \{#project-settings\}

Projects ページで各 Project カードにある三点リーダー `⋮` ボタンをクリックすると、
プロジェクトレベルの設定にアクセスできます。

![Project card menu](/screenshots/project_menu.png)

[Project Settings ページ](https://dashboard.convex.dev/project/settings)では、
次の操作が行えます:

* プロジェクトの名前とスラッグを更新する。
* プロジェクトの Admin を管理する。詳細は
  [Roles and Permissions](/dashboard/teams.md#roles-and-permissions) を参照。
* プロジェクトが消費した [usage metrics](/dashboard/teams.md#usage) の量を表示する。
* 本番デプロイメント用の
  [custom domains](/production/hosting/custom.mdx#custom-domains) を追加する。
* 本番およびプレビューのデプロイメント用のデプロイキーを生成する。
* [default environment variables](/production/environment-variables.mdx#project-environment-variable-defaults)
  を作成および編集する。
* `CONVEX_DEPLOYMENT` の設定値が分からなくなった場合に、プロジェクトへのアクセスを
  取り戻すための手順を表示する。
* プロジェクトを完全に削除する。

![Project settings](/screenshots/project_settings.png)

## プロジェクトの削除 \{#deleting-projects\}

プロジェクトを削除するには、Project カード上の三点リーダー `⋮` アイコンをクリックし、
「Delete」を選択します。プロジェクトは「Project Settings」ページから削除することもできます。

一度プロジェクトを削除すると、元に戻すことはできません。プロジェクトに関連付けられた
すべてのデプロイメントとデータは完全に削除されます。ダッシュボードからプロジェクトを
削除する際には、削除の確認が求められます。本番デプロイメントでアクティビティがある
プロジェクトを削除する場合は、誤って削除しないよう追加の確認ステップがあります。

![プロジェクトの削除](/screenshots/project_delete.png)