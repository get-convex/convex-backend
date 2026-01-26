---
id: "server.ValidatedFunction"
title: "インターフェース: ValidatedFunction<Ctx, ArgsValidator, Returns>"
custom_edit_url: null
---

[server](../modules/server.md).ValidatedFunction

**`非推奨`**

-- Convex 関数定義に使用される型については、
`MutationBuilder` などの型定義を参照してください。

引数検証つきの Convex クエリ、ミューテーション、またはアクション関数の定義。

引数検証により、この関数に渡される引数が期待どおりの型であることを保証できます。

例：

```js
import { query } from "./_generated/server";
import { v } from "convex/values";

export const func = query({
  args: {
    arg: v.string()
  },
  handler: ({ db }, { arg }) => {...},
});
```

**セキュリティのため、本番環境のアプリでは、すべての公開関数に引数バリデーションを追加することを推奨します。**

引数バリデーションを行わない関数については、[UnvalidatedFunction](../modules/server.md#unvalidatedfunction) を参照してください。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Ctx` | `Ctx` |
| `ArgsValidator` | extends [`PropertyValidators`](../modules/values.md#propertyvalidators) |
| `Returns` | `Returns` |

## プロパティ \{#properties\}

### args \{#args\}

• **args**: `ArgsValidator`

この関数の引数に対するバリデータです。

これは、引数名を [v](../modules/values.md#v) で構築したバリデータに対応付けるオブジェクトです。

```js
import { v } from "convex/values";

const args = {
  stringArg: v.string(),
  optionalNumberArg: v.optional(v.number()),
}
```

#### 定義元 \{#defined-in\}

[server/registration.ts:528](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L528)

***

### handler \{#handler\}

• **handler**: (`ctx`: `Ctx`, `args`: [`ObjectType`](../modules/values.md#objecttype)&lt;`ArgsValidator`&gt;) =&gt; `Returns`

#### 型定義 \{#type-declaration\}

▸ (`ctx`, `args`): `Returns`

この関数の実装です。

これは、適切なコンテキストと引数を受け取り、結果を返す関数です。

##### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `ctx` | `Ctx` | コンテキストオブジェクトです。関数のタイプに応じて QueryCtx、MutationCtx、または ActionCtx のいずれかになります。 |
| `args` | [`ObjectType`](../modules/values.md#objecttype)&lt;`ArgsValidator`&gt; | この関数の引数オブジェクトです。ArgsValidator で定義された型と一致します。 |

##### 戻り値 \{#returns\}

`Returns`

#### 定義場所 \{#defined-in\}

[server/registration.ts:542](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L542)