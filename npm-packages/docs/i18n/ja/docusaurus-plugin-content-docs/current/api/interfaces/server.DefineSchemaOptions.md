---
id: "server.DefineSchemaOptions"
title: "インターフェース: DefineSchemaOptions<StrictTableNameTypes>"
custom_edit_url: null
---

[server](../modules/server.md).DefineSchemaOptions

[defineSchema](../modules/server.md#defineschema) のオプション。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `StrictTableNameTypes` | extends `boolean` |

## プロパティ \{#properties\}

### schemaValidation \{#schemavalidation\}

• `Optional` **schemaValidation**: `boolean`

Convex が、すべてのドキュメントが定義したスキーマに一致しているかを
実行時に検証するかどうか。

`schemaValidation` が `true` の場合、Convex は次を実行します:

1. スキーマを push したときに、既存のすべてのドキュメントがスキーマに
   一致しているかをチェックします。
2. ミューテーションの実行中に行われるすべての挿入および更新がスキーマに
   一致しているかをチェックします。

`schemaValidation` が `false` の場合、Convex は新規および既存のドキュメントが
スキーマに一致しているかを検証しません。スキーマ固有の TypeScript 型は
引き続き生成されますが、ドキュメントがそれらの型に一致しているかどうかは
実行時には検証されません。

デフォルトでは、`schemaValidation` は `true` です。

#### 定義元 \{#defined-in\}

[server/schema.ts:727](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L727)

***

### strictTableNameTypes \{#stricttablenametypes\}

• `Optional` **strictTableNameTypes**: `StrictTableNameTypes`

TypeScript 型で、スキーマに存在しないテーブルへのアクセスを許可するかどうかを指定します。

`strictTableNameTypes` が `true` の場合、スキーマに定義されていないテーブルを使用すると
TypeScript のコンパイルエラーが発生します。

`strictTableNameTypes` が `false` の場合、スキーマに記載されていないテーブルにもアクセスでき、
そのドキュメント型は `any` になります。

`strictTableNameTypes: false` は、素早くプロトタイピングするときに便利です。

`strictTableNameTypes` の値に関わらず、スキーマはスキーマで定義されているテーブルの
ドキュメントだけを検証します。ダッシュボードや JavaScript のミューテーション内からは、
その他のテーブルも作成および変更できます。

デフォルトでは、`strictTableNameTypes` は `true` です。

#### 定義元 \{#defined-in\}

[server/schema.ts:746](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L746)