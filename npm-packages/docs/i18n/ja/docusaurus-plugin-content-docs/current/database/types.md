---
title: "データ型"
sidebar_position: 40
description: "Convex ドキュメントでサポートされているデータ型"
---

import ConvexValues from "@site/i18n/ja/docusaurus-plugin-content-docs/current/\_convexValues.mdx";

すべての Convex ドキュメントは JavaScript オブジェクトとして定義されます。これらのオブジェクトは、以下のいずれかの型のフィールド値を持つことができます。

テーブル内のドキュメントの構造は、
[スキーマを定義する](/database/schemas.mdx)
ことでコードとして明示できます。

## Convex values（値） \{#convex-values\}

<ConvexValues />

## システムフィールド \{#system-fields\}

Convex のすべてのドキュメントには、自動的に生成される 2 つのシステムフィールドがあります:

* `_id`: ドキュメントの[ドキュメント ID](/database/document-ids.mdx)。
* `_creationTime`: このドキュメントが作成された時刻（Unix エポックからのミリ秒）。

## 制限事項 \{#limits\}

Convex の値は合計サイズが 1MB 未満である必要があります。これは現時点ではおおよその制限ですが、この制限によく引っかかり、ドキュメントのサイズをより正確に計算する方法が必要な場合は、
[私たちに連絡してください](https://convex.dev/community)。ドキュメントにはネストされた値を含めることができ、オブジェクトや、他の Convex 型を含む配列などを持たせられます。Convex 型は最大でも 16 階層までしかネストできず、ネストされた値ツリー全体の累積サイズは 1MB の制限未満でなければなりません。

テーブル名には英数字（&quot;a&quot; から &quot;z&quot;、&quot;A&quot; から &quot;Z&quot;、&quot;0&quot; から &quot;9&quot;）およびアンダースコア（&quot;&#95;&quot;）を含めることができますが、アンダースコアで始めることはできません。

その他の制限については[こちら](/production/state/limits.mdx)を参照してください。

これらの制限のいずれかが要件に合わない場合は、
[ぜひお知らせください](https://convex.dev/community)!

## `undefined` の扱い \{#working-with-undefined\}

TypeScript の値 `undefined` は有効な Convex の値ではないため、Convex の関数の引数や戻り値、保存されるドキュメントの中では使用できません。

1. `undefined` を値として持つオブジェクト/レコードは、そのフィールドが存在しない場合と同じ扱いになります。`{a: undefined}` は、関数に渡したりデータベースに保存したりするときに `{}` に変換されます。Convex の関数呼び出しや Convex データベースは、データを `JSON.stringify` でシリアライズしていると考えることができ、`JSON.stringify` も同様に `undefined` の値を削除します。
2. オブジェクトフィールドのバリデータには、そのフィールドが存在しない可能性があることを示すために `v.optional(...)` を使えます。
   * オブジェクトのフィールド &quot;a&quot; が存在しない、つまり `const obj = {};` の場合、
     `obj.a === undefined` となります。これは TypeScript/JavaScript の性質であり、Convex 特有のものではありません。
3. フィルタやインデックスクエリで `undefined` を使用でき、そのフィールドを持たないドキュメントにマッチします。つまり、
   `.withIndex("by_a", q=>q.eq("a", undefined))` はドキュメント `{}` と
   `{b: 1}` にはマッチしますが、`{a: 1}` や `{a: null, b: 1}` にはマッチしません。
   * Convex の順序付けのルールでは、`undefined < null < その他すべての値` なので、
     `q.gte("a", null as any)` や `q.gt("a", undefined)` を使って、フィールドを *持っている* ドキュメントにマッチさせることができます。
4. `{a: undefined}` が `{}` と異なるのは、`ctx.db.patch` に渡した場合のただ 1 つのケースだけです。`{a: undefined}` を渡すとドキュメントからフィールド &quot;a&quot; が削除されますが、`{}` を渡してもフィールド &quot;a&quot; は変更されません。[既存ドキュメントの更新](/database/writing-data.mdx#updating-existing-documents)を参照してください。
5. `undefined` は関数の引数からは取り除かれますが、`ctx.db.patch` では意味を持つため、クライアントから patch の引数を渡すときにはいくつかの工夫が必要です。
   * クライアントが `args={}`（あるいは同等な `args={a: undefined}`）を渡したときにフィールド &quot;a&quot; を変更せずに残したい場合は、
     `ctx.db.patch(id, args)` を使います。
   * クライアントが `args={}` を渡したときにフィールド &quot;a&quot; を削除したい場合は、
     `ctx.db.patch(id, {a: undefined, ...args})` を使います。
   * クライアントが `args={}` を渡したときにはフィールド &quot;a&quot; を変更せず、`args={a: null}` を渡したときに削除したい場合は、次のようにできます。
     ```ts
     if (args.a === null) {
       args.a = undefined;
     }
     await ctx.db.patch(tableName, id, args);
     ```
6. プレーンな `undefined` / `void` を返す関数は、`null` を返したものとして扱われます。
7. `[undefined]` のように `undefined` を含む配列は、Convex の値として使用するとエラーになります。

`undefined` の特殊な挙動を避けたい場合は、代わりに `null` を使うことができます。`null` は有効な Convex の値です。

## 日付と時刻の扱い \{#working-with-dates-and-times\}

Convex には日付と時刻を扱うための特別なデータ型はありません。日付をどのように保存するかは、アプリケーションの要件によって異なります。

1. 単に「ある時点」だけを扱いたい場合は、
   [UTC タイムスタンプ](https://en.wikipedia.org/wiki/Unix_time) を保存するとよいでしょう。タイムスタンプをミリ秒単位の `number` として保存する `_creationTime` フィールドの例に従うことを推奨します。関数内やクライアント側では、そのタイムスタンプをコンストラクタに渡すことで JavaScript の `Date` を作成できます:
   `new Date(timeInMsSinceEpoch)`。その後、任意のタイムゾーン（ユーザーのマシンで設定されているタイムゾーンなど）で日付と時刻を表示できます。
   * 関数内で現在の UTC タイムスタンプを取得してデータベースに保存するには、`Date.now()` を使用します。
2. 予約アプリの実装など、カレンダー上の日付や特定の時刻が重要な場合は、実際の日付や時刻を文字列として保存するべきです。アプリが複数のタイムゾーンをサポートする場合は、タイムゾーンも一緒に保存する必要があります。[ISO8601](https://en.wikipedia.org/wiki/ISO_8601) は、`"2024-03-21T14:37:15Z"` のように日付と時刻を 1 つの文字列でまとめて保存する一般的な形式です。ユーザーが特定のタイムゾーンを選択できる場合は、通常は
   [IANA タイムゾーン名](https://en.wikipedia.org/wiki/Tz_database#Names_of_time_zones)
   を使って、別の `string` フィールドとして保存するのがよいでしょう（ただし、`"|"` のような一意な文字で 2 つのフィールドを連結してもかまいません）。

より高度な表示（フォーマット）や日付・時刻の操作を行うには、次のような一般的な JavaScript ライブラリのいずれかを使用してください: [date-fns](https://date-fns.org/),
[Day.js](https://day.js.org/), [Luxon](https://moment.github.io/luxon/),
[Moment.js](https://momentjs.com/)。