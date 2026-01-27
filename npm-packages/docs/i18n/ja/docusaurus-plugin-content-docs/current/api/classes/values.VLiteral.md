---
id: "values.VLiteral"
title: "クラス: VLiteral<Type, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VLiteral

`v.literal()` バリデータの型です。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `Type` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## 継承階層 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VLiteral`**

## コンストラクタ \{#constructors\}

### コンストラクター \{#constructor\}

• **new VLiteral**&lt;`Type`, `IsOptional`&gt;(`«destructured»`)

通常は `v.literal(value)` を使う方が一般的です。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `Type` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |
| › `value` | `Type` |

#### オーバーライド \{#overrides\}

BaseValidator&lt;Type, IsOptional&gt;.constructor

#### 定義場所 \{#defined-in\}

[values/validators.ts:441](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L441)

## プロパティ \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

TypeScript でのみ利用され、このバリデータで検証される JS の値の TypeScript 型です。

#### 継承元 \{#inherited-from\}

BaseValidator.type

#### 定義元 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

TypeScript のみ。これが Object バリデータであれば、
そのプロパティ名の TS 型になります。

#### 継承元 \{#inherited-from\}

BaseValidator.fieldPaths

#### 定義場所 \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

このバリデータがオブジェクトのオプションプロパティ用の値バリデータかどうか。

#### 継承元 \{#inherited-from\}

BaseValidator.isOptional

#### 定義元 \{#defined-in\}

[values/validators.ts:47](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L47)

***

### isConvexValidator \{#isconvexvalidator\}

• `Readonly` **isConvexValidator**: `true`

常に `"true"` です。

#### 継承元 \{#inherited-from\}

BaseValidator.isConvexValidator

#### 定義元 \{#defined-in\}

[values/validators.ts:52](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L52)

***

### value \{#value\}

• `Readonly` **value**: `Type`

検証済みの値が一致していなければならない値です。

#### 定義元 \{#defined-in\}

[values/validators.ts:431](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L431)

***

### kind \{#kind\}

• `Readonly` **kind**: `"literal"`

バリデータの種類で、`"literal"` です。

#### 定義元 \{#defined-in\}

[values/validators.ts:436](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L436)