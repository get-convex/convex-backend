---
id: "values.VArray"
title: "クラス: VArray<Type, Element, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VArray

`v.array()` バリデーターの型です。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `Type` |
| `Element` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## 継承階層 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VArray`**

## コンストラクタ \{#constructors\}

### constructor \{#constructor\}

• **new VArray**&lt;`Type`, `Element`, `IsOptional`&gt;(`«destructured»`)

通常は代わりに `v.array(element)` を使用します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `Type` |
| `Element` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |
| › `element` | `Element` |

#### オーバーライド \{#overrides\}

BaseValidator&lt;Type, IsOptional&gt;.constructor

#### 定義場所 \{#defined-in\}

[values/validators.ts:490](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L490)

## プロパティ \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

TypeScript の場合にのみ使用される、このバリデーターで検証される JS 値の TS 型です。

#### 継承元 \{#inherited-from\}

BaseValidator.type

#### 定義場所 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

TypeScript のみ。これが Object バリデータである場合、
そのプロパティ名の TS 型になります。

#### 継承元 \{#inherited-from\}

BaseValidator.fieldPaths

#### 定義場所 \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

これがオプションのオブジェクトプロパティ値のバリデータかどうかを示します。

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

### element \{#element\}

• `Readonly` **element**: `Element`

配列要素用のバリデーター。

#### 定義元 \{#defined-in\}

[values/validators.ts:480](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L480)

***

### kind \{#kind\}

• `Readonly` **kind**: `"array"`

バリデーターの種類を示し、値は `"array"` です。

#### 定義場所 \{#defined-in\}

[values/validators.ts:485](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L485)