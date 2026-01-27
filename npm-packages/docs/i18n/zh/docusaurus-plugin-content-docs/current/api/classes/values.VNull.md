---
id: "values.VNull"
title: "类：VNull<Type, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VNull

`v.null()` 验证器的类型。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `null` |
| `IsOptional` | 约束为 [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## 层次结构 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VNull`**

## 构造函数 \{#constructors\}

### 构造函数 \{#constructor\}

• **new VNull**&lt;`Type`, `IsOptional`&gt;(`«destructured»`)

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `null` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |

#### 继承自 \{#inherited-from\}

BaseValidator&lt;Type, IsOptional&gt;.constructor

#### 定义于 \{#defined-in\}

[values/validators.ts:54](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L54)

## 属性 \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

仅适用于 TypeScript，即此验证器所验证的 JS 值对应的 TS 类型。

#### 继承自 \{#inherited-from\}

BaseValidator.type

#### 定义在 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

仅适用于 TypeScript，如果这是一个 Object 校验器，那么
该字段表示其属性名的 TS 类型。

#### 继承自 \{#inherited-from\}

BaseValidator.fieldPaths

#### 定义于 \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

此验证器是否用于可选的对象属性值。

#### 继承自 \{#inherited-from\}

BaseValidator.isOptional

#### 定义于 \{#defined-in\}

[values/validators.ts:47](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L47)

***

### isConvexValidator \{#isconvexvalidator\}

• `Readonly` **isConvexValidator**: `true`

始终为 `"true"`。

#### 继承自 \{#inherited-from\}

BaseValidator.isConvexValidator

#### 定义于 \{#defined-in\}

[values/validators.ts:52](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L52)

***

### kind \{#kind\}

• `Readonly` **kind**: `"null"`

验证器的类型，`"null"`。

#### 定义于 \{#defined-in\}

[values/validators.ts:238](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L238)