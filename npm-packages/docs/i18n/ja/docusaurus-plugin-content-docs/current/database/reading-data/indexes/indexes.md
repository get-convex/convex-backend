---
title: "インデックス"
sidebar_position: 100
description: "データベースインデックスでクエリを高速化する"
---

インデックスは、Convex にドキュメントの整理方法を指示することで
[ドキュメントに対するクエリ](/database/reading-data/reading-data.mdx#querying-documents)
を高速化できるデータ構造です。インデックスを使うと、
クエリ結果におけるドキュメントの並び順も変更できます。

インデックスについてのより詳しい解説は、
[Indexes and Query Performance](/database/reading-data/indexes/indexes-and-query-perf.md)
を参照してください。

## インデックスの定義 \{#defining-indexes\}

インデックスは Convex の[スキーマ](/database/schemas.mdx)の一部として定義されます。各インデックスは次の要素で構成されます。

1. 名前
   * テーブルごとにユニークでなければなりません。
2. インデックスするフィールドの順序付きリスト
   * ネストされたドキュメント上のフィールドを指定するには、
     `properties.name` のようなドット区切りのパスを使用します。

テーブルにインデックスを追加するには、そのテーブルのスキーマで
[`index`](/api/classes/server.TableDefinition#index) メソッドを使用します。

```ts noDialect title="convex/schema.ts"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

// 2つのインデックスを持つmessagesテーブルを定義します。
export default defineSchema({
  messages: defineTable({
    channel: v.id("channels"),
    body: v.string(),
    user: v.id("users"),
  })
    .index("by_channel", ["channel"])
    .index("by_channel_user", ["channel", "user"]),
});
```

`by_channel` インデックスは、スキーマで定義された `channel` フィールドを基準に並び替えられます。
同じチャンネル内のメッセージは、すべてのインデックスに自動的に追加される
[システム生成フィールド `_creationTime`](/database/types.md#system-fields)
によって順序付けられます。

一方で、`by_channel_user` インデックスは、同じ `channel` 内のメッセージを、
まず送信した `user` ごとに並び替え、そのうえで `_creationTime` によって順序付けます。

インデックスは
[`npx convex dev`](/cli.md#run-the-convex-dev-server) と
[`npx convex deploy`](/cli.md#deploy-convex-functions-to-production)
の実行時に作成されます。

最初にインデックスを定義してデプロイする際は、通常より少し時間がかかることに
気付くかもしれません。これは Convex がインデックスを *バックフィル* する必要が
あるためです。テーブル内のデータが多いほど、Convex がインデックス順に整理するのに
時間がかかります。大きなテーブルにインデックスを追加する必要がある場合は、
[段階的インデックス](#staged-indexes) を使用してください。

インデックスを定義したのと同じデプロイ内で、そのインデックスに対してクエリを実行しても問題ありません。
Convex は、新しいクエリやミューテーション関数が登録される前に、そのインデックスが
バックフィルされていることを保証します。

<Admonition type="caution" title="インデックスの削除には注意してください">
  新しいインデックスの追加に加えて、`npx convex deploy` はスキーマ内に存在しなくなった
  インデックスを削除します。スキーマからインデックスを削除する前に、そのインデックスが
  完全に未使用であることを必ず確認してください！
</Admonition>

## インデックスを使ったドキュメントのクエリ \{#querying-documents-using-indexes\}

`by_channel` インデックスに対する「1〜2 分前に作成された `channel` 内のメッセージ」というクエリは次のようになります。

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel", (q) =>
    q
      .eq("channel", channel)
      .gt("_creationTime", Date.now() - 2 * 60000)
      .lt("_creationTime", Date.now() - 60000),
  )
  .collect();
```

[`.withIndex`](/api/interfaces/server.QueryInitializer#withindex) メソッドは、
どのインデックスに対してクエリを実行し、Convex がそのインデックスを使って
どのようにドキュメントを選択するかを定義します。最初の引数はインデックス名、
2 つ目の引数は *インデックス範囲式* です。インデックス範囲式とは、クエリを実行するときに
Convex がどのドキュメントを検討対象とするかを表現したものです。

どのインデックスを選ぶかによって、インデックス範囲式の書き方と、
結果が返される順序の両方に影響します。たとえば、`by_channel` インデックスと
`by_channel_user` インデックスの両方を作成しておけば、チャンネル内の結果を
`_creationTime` でソートすることも、`user` でソートすることもできます。それでは、
`by_channel_user` インデックスを次のように使ったとします:

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel_user", (q) => q.eq("channel", channel))
  .collect();
```

結果は、`user`、次に `_creationTime` の順で並んだ、ある `channel` 内にあるすべてのメッセージになります。`by_channel_user` を次のように使うと、こうなります:

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel_user", (q) =>
    q.eq("channel", channel).eq("user", user),
  )
  .collect();
```

結果として得られるのは、指定された `channel` で `user` が送信したメッセージで、
`_creationTime` の順に並んだものになります。

インデックス範囲表現は常に次のものを連ねたリストです:

1. [`.eq`](/api/interfaces/server.IndexRangeBuilder#eq) で定義される、
   0 個以上の等値条件。
2. [任意] [`.gt`](/api/interfaces/server.IndexRangeBuilder#gt) または
   [`.gte`](/api/interfaces/server.IndexRangeBuilder#gte) で定義される下限条件。
3. [任意] [`.lt`](/api/interfaces/server.IndexRangeBuilder#lt) または
   [`.lte`](/api/interfaces/server.IndexRangeBuilder#lte) で定義される上限条件。

**インデックスのフィールドは、定義された順番で順にたどらなければなりません。**

各等値条件は、インデックス内の異なるフィールドを、先頭から順番に比較する必要があります。
下限および上限の条件は等値条件に続き、その次のフィールドを比較しなければなりません。

たとえば、次のようなクエリを書くことはできません:

```ts
// コンパイルできません!
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel", (q) =>
    q
      .gt("_creationTime", Date.now() - 2 * 60000)
      .lt("_creationTime", Date.now() - 60000),
  )
  .collect();
```

このクエリは無効です。というのも、`by_channel` インデックスは
`(channel, _creationTime)` の順でソートされており、このクエリの範囲指定では
`channel` を単一の値に絞り込む前に `_creationTime` に対して比較を行っているためです。
インデックスはまず `channel`、次に `_creationTime` の順でソートされているので、
1〜2分前に作成された全てのチャンネルにまたがるメッセージを探すインデックスとしては役に立ちません。
`withIndex` 内の TypeScript 型定義が、この点についての正しい書き方を案内してくれます。

どのインデックスに対してどのようなクエリを実行できるかをよりよく理解するには、
[Introduction to Indexes and Query Performance](/database/reading-data/indexes/indexes-and-query-perf.md)
を参照してください。

**クエリの実行性能は、範囲指定の具体性に依存します。**

たとえば、クエリが次のような場合を考えます。

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel", (q) =>
    q
      .eq("channel", channel)
      .gt("_creationTime", Date.now() - 2 * 60000)
      .lt("_creationTime", Date.now() - 60000),
  )
  .collect();
```

すると、そのクエリのパフォーマンスは、1〜2分前に作成された `channel` 内のメッセージ数に依存することになります。

インデックス範囲が指定されていない場合、インデックス内のすべてのドキュメントが
クエリの対象になります。

<Admonition type="tip" title="良いインデックス範囲の選び方">
  パフォーマンスのために、インデックス範囲はできるだけ具体的に定義しましょう！ 大きなテーブルに対してクエリを実行していて、
  `.eq` を使った等価条件を追加できない場合は、新しいインデックスの定義を検討してください。
</Admonition>

`.withIndex` は、Convex がそのインデックスを効率的に使って検索できる範囲だけを指定できるように設計されています。
それ以外のフィルタリングには、[`.filter`](/api/interfaces/server.Query#filter) メソッドを使ってください。

たとえば「`channel` 内で **自分が作成していない** メッセージ」をクエリしたい場合、次のようにできます。

```ts
const messages = await ctx.db
  .query("messages")
  .withIndex("by_channel", (q) => q.eq("channel", channel))
  .filter((q) => q.neq(q.field("user"), myUserId))
  .collect();
```

この場合、このクエリのパフォーマンスはチャンネル内のメッセージ数によって決まります。Convex はチャンネル内の各メッセージを順番に確認し、`user` フィールドが `myUserId` と一致しないメッセージだけを返します。

## インデックスによるソート \{#sorting-with-indexes\}

`withIndex` を使用するクエリは、そのインデックスで指定された列によって並び替えられます。

インデックス内の列の順序が、ソートの優先度を決定します。
インデックスで最初に列挙された列の値が最初に比較されます。
以降の列は、それ以前のすべての列が一致した場合にのみ、タイブレーカーとして比較されます。

Convex はすべてのインデックスの最後の列として `_creationTime` を自動的に含めるため、インデックス内の他のすべての列が等しい場合、常に `_creationTime` が最終的なタイブレーカーになります。

たとえば、`by_channel_user` には `channel`、`user`、`_creationTime` が含まれます。
したがって、`.withIndex("by_channel_user")` を使用する `messages` へのクエリは、まず channel、次に各 channel 内で user、最後に作成時刻の順にソートされます。

インデックスによるソートを使うと、スコア上位 `N` 人のユーザー、直近の `N` 件のトランザクション、「いいね」が多い順の上位 `N` 件のメッセージの表示といったユースケースを満たすことができます。

たとえば、ゲーム内でスコアの高いプレイヤー上位 10 人を取得するには、プレイヤーの最高スコアに対するインデックスを定義します。

```ts
export default defineSchema({
  players: defineTable({
    username: v.string(),
    highestScore: v.number(),
  }).index("by_highest_score", ["highestScore"]),
});
```

その後は、インデックスと [`take(10)`](/api/interfaces/server.Query#take) を使えば、スコアが最も高いプレイヤー上位 10 人を効率的に検索できます。

```ts
const topScoringPlayers = await ctx.db
  .query("users")
  .withIndex("by_highest_score")
  .order("desc")
  .take(10);
```

この例では、歴代でスコアが最も高いプレイヤーを探しているため、範囲式は省略されています。`take()` を使っているため、このクエリは大きなデータセットに対しても比較的効率的です。

範囲式なしでインデックスを使う場合、必ず `withIndex` と一緒に次のいずれかを使用してください:

1. [`.first()`](/api/interfaces/server.Query#first)
2. [`.unique()`](/api/interfaces/server.Query#unique)
3. [`.take(n)`](/api/interfaces/server.Query#take)
4. [`.paginate(ops)`](/database/pagination.mdx)

これらの API によって、テーブル全体をスキャンすることなく、クエリの結果を妥当なサイズに効率よく制限できます。

<Admonition type="caution" title="フルテーブルスキャン">
  クエリがデータベースからドキュメントを取得する際、指定した範囲内の行をスキャンします。たとえば `.collect()` を使っている場合、その範囲内のすべての行をスキャンします。したがって、範囲式なしで `withIndex` を使うと
  [テーブル全体をスキャンする](https://docs.convex.dev/database/indexes/indexes-and-query-perf#full-table-scans)
  ことになり、テーブルに何千行もある場合は低速になる可能性があります。`.filter()` はどのドキュメントがスキャンされるかには影響しません。`.first()` または `.unique()`、`.take(n)` を使うと、必要なドキュメント数に達するまでしか行をスキャンしません。
</Admonition>

より的を絞ったクエリに対応するために、範囲式を含めることもできます。たとえば、カナダでスコア上位のプレイヤーを取得するには、`take()` と範囲式の両方を使うことができます:

```ts
// カナダで最高得点の上位10人のプレイヤーをクエリします。
const topScoringPlayers = await ctx.db
  .query("users")
  .withIndex("by_country_highest_score", (q) => q.eq("country", "CA"))
  .order("desc")
  .take(10);
```

## ステージドインデックス \{#staged-indexes\}

デフォルトでは、インデックスの作成はコードをデプロイしたタイミングで同期的に行われます。大きな
テーブルでは、既存テーブルに対して
[インデックスをバックフィルする](indexes-and-query-perf#backfilling-and-maintaining-indexes)
処理が遅くなることがあります。ステージドインデックスは、大きなテーブルに対してデプロイをブロックせずに非同期でインデックスを作成する仕組みです。これは、複数の機能を同時に開発している場合に便利です。

ステージドインデックスを作成するには、`schema.ts` で次の構文を使用します。

```ts
export default defineSchema({
  messages: defineTable({
    channel: v.id("channels"),
  }).index("by_channel", { fields: ["channel"], staged: true }),
});
```

<Admonition type="caution" title="有効化されるまでステージング中のインデックスは使用できません">
  ステージング中のインデックスは、有効化されるまでクエリで使用できません。有効化するには、
  まずバックフィルが完了している必要があります。
</Admonition>

バックフィルの進行状況は、ダッシュボードのデータページにある
[*Indexes* ペイン](/dashboard/deployments/data/#view-the-indexes-of-a-table)から確認できます。完了したら、`staged` オプションを削除してインデックスを有効化し、クエリで使用できるようにします。

```ts
export default defineSchema({
  messages: defineTable({
    channel: v.id("channels"),
  }).index("by_channel", { fields: ["channel"] }),
});
```

## 制限 \{#limits\}

Convex は最大 16 個のフィールドを含むインデックスをサポートします。各テーブルには最大 32 個の
インデックスを定義できます。インデックスに同じフィールドを重複して含めることはできません。

インデックスには予約済みフィールド（`_` で始まるもの）は使用できません。
安定した順序付けを保証するために、`_creationTime` フィールドがすべてのインデックスの末尾に自動的に追加されます。
これはインデックス定義で明示的に追加すべきではなく、インデックスのフィールド数の上限に含まれます。

`by_creation_time` インデックスは自動的に作成されます（インデックスを指定しない
データベースのクエリで使用されるインデックスです）。`by_id` インデックスは予約されています。