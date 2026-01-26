---
id: "values.VUnion"
title: "クラス: VUnion<Type, T, IsOptional, FieldPaths>"
custom_edit_url: null
---

[values](../modules/values.md).VUnion

`v.union()` バリデータの型です。

## 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `Type` |
| `T` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt;[] |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `T`[`number`][`"fieldPaths"`] |

## 継承階層 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`, `FieldPaths`&gt;

  ↳ **`VUnion`**

## コンストラクター \{#constructors\}

### constructor \{#constructor\}

• **new VUnion**&lt;`Type`, `T`, `IsOptional`, `FieldPaths`&gt;(`«destructured»`)

通常は `v.union(...members)` を使用してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `Type` |
| `T` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt;[] |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `T`[`number`][`"fieldPaths"`] |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |
| › `members` | `T` |

#### オーバーライド \{#overrides\}

BaseValidator&lt;Type, IsOptional, FieldPaths&gt;.constructor

#### 定義場所 \{#defined-in\}

[values/validators.ts:619](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L619)

## プロパティ \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

TypeScript 専用で、このバリデータが検証する JS の値の TS 型です。

#### 継承元 \{#inherited-from\}

BaseValidator.type

#### 定義場所 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `FieldPaths`

TypeScript の場合にのみ利用できます。これが Object バリデーターである場合、そのプロパティ名の TypeScript 型です。

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

#### 定義場所 \{#defined-in\}

[values/validators.ts:47](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L47)

***

### isConvexValidator \{#isconvexvalidator\}

• `Readonly` **isConvexValidator**: `true`

常に `"true"` です。

#### 継承元 \{#inherited-from\}

BaseValidator.isConvexValidator

#### 定義場所 \{#defined-in\}

[values/validators.ts:52](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L52)

***

### members \{#members\}

• `Readonly` **members**: `T`

バリデータの配列であり、そのいずれか1つが値にマッチしている必要があります。

#### 定義元 \{#defined-in\}

[values/validators.ts:609](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L609)

***

### kind \{#kind\}

• `Readonly` **kind**: `"union"`

バリデーターの種類で、`"union"` です。

#### 定義場所 \{#defined-in\}

[values/validators.ts:614](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L614)