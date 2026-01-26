---
id: "values.VBoolean"
title: "クラス: VBoolean<Type, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VBoolean

`v.boolean()` バリデーターの型。

## 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `boolean` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## 継承階層 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VBoolean`**

## コンストラクタ \{#constructors\}

### コンストラクター \{#constructor\}

• **new VBoolean**&lt;`Type`, `IsOptional`&gt;(`«destructured»`)

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `boolean` |
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

TypeScript の場合のみ使用でき、このバリデータによって検証される JS の値に対応する TS 型です。

#### 継承元 \{#inherited-from\}

BaseValidator.type

#### 定義元 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

TypeScript の場合にのみ有効です。これが Object バリデータである場合、
そのプロパティ名の TypeScript 型になります。

#### 継承元 \{#inherited-from\}

BaseValidator.fieldPaths

#### 定義場所 \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

これがオプションのオブジェクトプロパティ用の値バリデータかどうかを示します。

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

• `Readonly` **kind**: `"boolean"`

バリデータの種類（`"boolean"`）。

#### 定義元 \{#defined-in\}

[values/validators.ts:168](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L168)