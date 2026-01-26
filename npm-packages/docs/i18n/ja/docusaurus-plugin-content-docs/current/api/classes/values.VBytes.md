---
id: "values.VBytes"
title: "クラス: VBytes<Type, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VBytes

`v.bytes()` バリデータの型です。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `ArrayBuffer` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## 継承階層 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VBytes`**

## コンストラクター \{#constructors\}

### コンストラクター \{#constructor\}

• **new VBytes**&lt;`Type`, `IsOptional`&gt;(`«destructured»`)

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `ArrayBuffer` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |

#### 継承元 \{#inherited-from\}

BaseValidator&lt;Type, IsOptional&gt;.constructor

#### 定義元 \{#defined-in\}

[values/validators.ts:54](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L54)

## プロパティ \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

TypeScript のみで使用される、このバリデーターによって検証される JS の値の TS 型です。

#### 継承元 \{#inherited-from\}

BaseValidator.type

#### 定義箇所 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

TypeScript の場合のみ、これが Object バリデータであれば、
そのプロパティ名の TypeScript 型になります。

#### 継承元 \{#inherited-from\}

BaseValidator.fieldPaths

#### 定義元 \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

これが省略可能なオブジェクトプロパティ値用のバリデータかどうかを示します。

#### 継承元 \{#inherited-from\}

BaseValidator.isOptional

#### 定義元 \{#defined-in\}

[values/validators.ts:47](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L47)

***

### isConvexValidator \{#isconvexvalidator\}

• `Readonly` **isConvexValidator**: `true`

常に `"true"` となります。

#### 継承元 \{#inherited-from\}

BaseValidator.isConvexValidator

#### 定義場所 \{#defined-in\}

[values/validators.ts:52](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L52)

***

### kind \{#kind\}

• `Readonly` **kind**: `"bytes"`

バリデータの種類。値は `"bytes"` です。

#### 定義元 \{#defined-in\}

[values/validators.ts:192](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L192)