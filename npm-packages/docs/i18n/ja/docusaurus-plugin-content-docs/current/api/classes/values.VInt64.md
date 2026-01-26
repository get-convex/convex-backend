---
id: "values.VInt64"
title: "クラス: VInt64<Type, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VInt64

`v.int64()` バリデーターの型です。

## 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `bigint` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## 継承階層 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VInt64`**

## コンストラクタ \{#constructors\}

### コンストラクタ \{#constructor\}

• **new VInt64**&lt;`Type`, `IsOptional`&gt;(`«destructured»`)

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `bigint` |
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

TypeScript でのみ使用され、この validator によって検証される JS の値の TS 型です。

#### 継承元 \{#inherited-from\}

BaseValidator.type

#### 定義元 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

TypeScript の場合のみ、これがオブジェクトバリデータであれば、
そのプロパティ名の TS 型になります。

#### 継承元 \{#inherited-from\}

BaseValidator.fieldPaths

#### 定義箇所 \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

これが任意のオブジェクトプロパティ値バリデータかどうかを示します。

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

• `Readonly` **kind**: `"int64"`

バリデータの種類で、`"int64"` です。

#### 定義元 \{#defined-in\}

[values/validators.ts:145](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L145)