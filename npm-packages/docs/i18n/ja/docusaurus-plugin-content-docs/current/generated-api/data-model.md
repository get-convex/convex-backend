---
title: "dataModel.d.ts"
sidebar_position: 1
description: "データベーススキーマとドキュメント用に生成された TypeScript 型"
---

<Admonition type="caution" title="このコードは自動生成されています">
  これらのエクスポートは `convex` パッケージから直接利用することはできません。

  代わりに `npx convex dev` を実行して
  `convex/_generated/dataModel.d.ts` を作成する必要があります。
</Admonition>

自動生成されたデータモデル型です。

## 型 \{#types\}

### TableNames \{#tablenames\}

Ƭ **TableNames**: `string`

Convex 上のすべてのテーブル名。

***

### Doc \{#doc\}

Ƭ **Doc**`<TableName>`: `Object`

Convex に格納されているドキュメントの型です。

#### 型パラメータ \{#type-parameters\}

| 名前        | 型                                  | 説明                                                   |
| :---------- | :---------------------------------- | :----------------------------------------------------- |
| `TableName` | extends [`TableNames`](#tablenames) | テーブル名を表す文字列リテラル型（例: `"users"`）。    |
------------------------------------------------------------------------------------------

### Id

Convex 内のドキュメントを指す識別子です。

Convex のドキュメントは `Id` によって一意に識別され、その `Id` は `_id` フィールドから
参照できます。詳しくは [Document IDs](/database/document-ids.mdx) を参照してください。

ドキュメントは、クエリ関数およびミューテーション関数内で `db.get(tableName, id)` を使って
読み込むことができます。

ID は実行時には単なる文字列ですが、この型を使うことで型チェック時に他の文字列と区別できます。

これは、データモデルに合わせて型付けされた [`GenericId`](/api/modules/values#genericid)
のエイリアスです。

#### 型パラメーター

| Name        | Type                                | Description                                             |
| :---------- | :---------------------------------- | :------------------------------------------------------ |
| `TableName` | extends [`TableNames`](#tablenames) | テーブル名を表す文字列リテラル型（例: &quot;users&quot;）。 |

***

### DataModel

Ƭ **DataModel**: `Object`

Convex のデータモデルを表す型です。

この型には、どのテーブルがあるか、そのテーブルに保存されているドキュメントの型、およびそれらに定義されたインデックスに関する情報が含まれます。

この型は
[`queryGeneric`](/api/modules/server#querygeneric) や
[`mutationGeneric`](/api/modules/server#mutationgeneric) といったメソッドをパラメータ化し、型安全にするために使用されます。