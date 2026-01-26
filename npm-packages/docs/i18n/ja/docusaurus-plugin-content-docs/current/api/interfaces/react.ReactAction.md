---
id: "react.ReactAction"
title: "インターフェース: ReactAction<Action>"
custom_edit_url: null
---

[react](../modules/react.md).ReactAction

サーバー上で Convex のアクションを実行するためのインターフェースです。

## 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"action"`&gt; |

## 呼び出し可能 \{#callable\}

### ReactAction \{#reactaction\}

▸ **ReactAction**(`...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

関数をサーバー側で実行し、その戻り値を表す `Promise` を返します。

#### Parameters \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Action`&gt; | サーバーに渡される関数の引数。 |

#### 戻り値 \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

サーバー側の関数呼び出しから返される値。

#### 定義元 \{#defined-in\}

[react/client.ts:136](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L136)