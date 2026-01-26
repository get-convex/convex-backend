---
title: 生成コード
description:
  "あなたのアプリの API に特化した自動生成の JavaScript と TypeScript コード"
---

Convex は、アプリのデータモデルと API に特化したコードを生成するためにコード生成を行います。Convex は TypeScript の型定義ファイル (`.d.ts`) を備えた JavaScript ファイル (`.js`) を生成します。

Convex を利用するのにコード生成は必須ではありませんが、生成されたコードを使うことでエディタでの補完機能がさらに向上し、TypeScript を使用している場合は型安全性も高まります。

コードを生成するには、次を実行します:

```
npx convex dev
```

これによって `convex/_generated` ディレクトリが作成され、その中には次のファイルが含まれます：

* [`api.js` と `api.d.ts`](./api.md)
* [`dataModel.d.ts`](./data-model.md)
* [`server.js` と `server.d.ts`](./server.md)
