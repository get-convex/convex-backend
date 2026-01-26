---
id: "values.VFloat64"
title: "クラス: VFloat64<Type, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VFloat64

`v.float64()` バリデータの型。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `number` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## 継承階層 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VFloat64`**

## コンストラクター \{#constructors\}

### コンストラクター \{#constructor\}

• **new VFloat64**&lt;`Type`, `IsOptional`&gt;(`«destructured»`)

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `number` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

#### パラメータ \{#parameters\}

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

TypeScript 専用で、このバリデータによって検証される JS の値の TS 型を表します。

#### 継承元 \{#inherited-from\}

BaseValidator.type

#### 定義元 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

TypeScript のみ有効。このバリデーターが Object バリデーターの場合、
これはそのプロパティ名の TypeScript 型です。

#### 継承元 \{#inherited-from\}

BaseValidator.fieldPaths

#### 定義元 \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

これがオプションのオブジェクトプロパティ用の値バリデータかどうかを示します。

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

### kind \{#kind\}

• `Readonly` **kind**: `"float64"`

バリデーターの種別で、値は `"float64"` です。

#### 定義元 \{#defined-in\}

[values/validators.ts:120](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L120)