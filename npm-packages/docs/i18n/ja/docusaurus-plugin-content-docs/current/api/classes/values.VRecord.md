---
id: "values.VRecord"
title: "クラス: VRecord<Type, Key, Value, IsOptional, FieldPaths>"
custom_edit_url: null
---

[values](../modules/values.md).VRecord

`v.record()` バリデータの型。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `Type` |
| `Key` | extends [`Validator`](../modules/values.md#validator)&lt;`string`, `"required"`, `any`&gt; |
| `Value` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `string` |

## 継承階層 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`, `FieldPaths`&gt;

  ↳ **`VRecord`**

## コンストラクター \{#constructors\}

### constructor \{#constructor\}

• **new VRecord**&lt;`Type`, `Key`, `Value`, `IsOptional`, `FieldPaths`&gt;(`«destructured»`)

通常はコンストラクタではなく `v.record(key, value)` を使用します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `Type` |
| `Key` | extends [`Validator`](../modules/values.md#validator)&lt;`string`, `"required"`, `any`&gt; |
| `Value` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `string` |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |
| › `key` | `Key` |
| › `value` | `Value` |

#### オーバーライド \{#overrides\}

BaseValidator&lt;Type, IsOptional, FieldPaths&gt;.constructor

#### 定義元 \{#defined-in\}

[values/validators.ts:547](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L547)

## プロパティ \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

TypeScript 専用で、このバリデータが検証する JS の値に対応する TS 型です。

#### 継承元 \{#inherited-from\}

BaseValidator.type

#### 定義元 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `FieldPaths`

TypeScript の場合にのみ、これが Object バリデータであれば、
そのプロパティ名の TypeScript 型になります。

#### 継承元 \{#inherited-from\}

BaseValidator.fieldPaths

#### 定義場所 \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

このバリデーターがオブジェクトのプロパティ値を省略可能として扱うかどうかを示します。

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

### key \{#key\}

• `Readonly` **key**: `Key`

レコードのキー用のバリデータです。

#### 定義場所 \{#defined-in\}

[values/validators.ts:532](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L532)

***

### value \{#value\}

• `Readonly` **value**: `Value`

レコード内の値用のバリデータです。

#### 定義元 \{#defined-in\}

[values/validators.ts:537](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L537)

***

### kind \{#kind\}

• `Readonly` **kind**: `"record"`

バリデータの種類で、`"record"` です。

#### 定義場所 \{#defined-in\}

[values/validators.ts:542](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L542)