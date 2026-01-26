---
id: "values.VId"
title: "クラス: VId<Type, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VId

`v.id(tableName)` バリデータの型です。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `Type` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## 継承関係 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VId`**

## コンストラクタ \{#constructors\}

### constructor \{#constructor\}

• **new VId**&lt;`Type`, `IsOptional`&gt;(`«destructured»`)

通常は `v.id(tableName)` を使用するのが一般的です。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `Type` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |
| › `tableName` | `TableNameFromType`&lt;`Type`&gt; |

#### オーバーライド元 \{#overrides\}

BaseValidator&lt;Type, IsOptional&gt;.constructor

#### 定義元 \{#defined-in\}

[values/validators.ts:84](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L84)

## プロパティ \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

TypeScript の場合にのみ、このバリデーターによって検証される JS の値の TS 型です。

#### 継承元 \{#inherited-from\}

BaseValidator.type

#### 定義元 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

TypeScript のみ。これが Object バリデータの場合、
そのプロパティ名の TypeScript 型になります。

#### 継承元 \{#inherited-from\}

BaseValidator.fieldPaths

#### 定義場所 \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

これが任意のオブジェクトプロパティ値用のバリデーターかどうかを示します。

#### 継承元 \{#inherited-from\}

BaseValidator.isOptional

#### 定義元 \{#defined-in\}

[values/validators.ts:47](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L47)

***

### isConvexValidator \{#isconvexvalidator\}

• `Readonly` **isConvexValidator**: `true`

常に `"true"`。

#### 継承元 \{#inherited-from\}

BaseValidator.isConvexValidator

#### 定義元 \{#defined-in\}

[values/validators.ts:52](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L52)

***

### tableName \{#tablename\}

• `Readonly` **tableName**: `TableNameFromType`&lt;`Type`&gt;

検証対象の Id が属していなければならないテーブル名。

#### 定義場所 \{#defined-in\}

[values/validators.ts:74](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L74)

***

### kind \{#kind\}

• `Readonly` **kind**: `"id"`

バリデータの種類は `"id"` です。

#### 定義元 \{#defined-in\}

[values/validators.ts:79](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L79)