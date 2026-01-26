---
id: "values.VNull"
title: "クラス: VNull<Type, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VNull

`v.null()` バリデーターの型です。

## 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `null` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## 継承階層 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VNull`**

## コンストラクタ \{#constructors\}

### コンストラクタ \{#constructor\}

• **new VNull**&lt;`Type`, `IsOptional`&gt;(`«destructured»`)

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `null` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |

#### 継承元 \{#inherited-from\}

BaseValidator&lt;Type, IsOptional&gt;.constructor

#### 定義場所 \{#defined-in\}

[values/validators.ts:54](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L54)

## プロパティ \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

TypeScript のみで利用され、このバリデーターが検証する JS の値に対応する TypeScript の型です。

#### 継承元 \{#inherited-from\}

BaseValidator.type

#### 定義場所 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

TypeScript のみ対象。これが Object バリデータの場合、
そのプロパティ名の TypeScript 型です。

#### 継承元 \{#inherited-from\}

BaseValidator.fieldPaths

#### 定義場所 \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

これがオプショナルなオブジェクトプロパティ値用のバリデーターかどうかを表します。

#### 継承元 \{#inherited-from\}

BaseValidator.isOptional

#### 定義場所 \{#defined-in\}

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

### kind \{#kind\}

• `Readonly` **kind**: `"null"`

バリデータの種別（`"null"`）。

#### 定義場所 \{#defined-in\}

[values/validators.ts:238](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L238)