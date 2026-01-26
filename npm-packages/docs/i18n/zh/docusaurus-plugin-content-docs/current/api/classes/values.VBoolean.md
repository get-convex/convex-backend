---
id: "values.VBoolean"
title: "类：VBoolean<Type, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VBoolean

`v.boolean()` 校验器的类型。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `boolean` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## 继承层次结构 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VBoolean`**

## 构造函数 \{#constructors\}

### 构造函数 \{#constructor\}

• **new VBoolean**&lt;`Type`, `IsOptional`&gt;(`«destructured»`)

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `boolean` |
| `IsOptional` | 继承自 [`OptionalProperty`](../modules/values.md#optionalproperty)，默认为 `"required"` |

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

仅适用于 TypeScript，即此验证器所校验 JS 值的 TS 类型。

#### 继承自 \{#inherited-from\}

BaseValidator.type

#### 定义于 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

仅适用于 TypeScript，如果这是一个对象验证器，则
这是其属性名称的 TypeScript 类型。

#### 继承自 \{#inherited-from\}

BaseValidator.fieldPaths

#### 定义于 \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

指示该对象属性值验证器是否为可选。

#### 继承自 \{#inherited-from\}

BaseValidator.isOptional

#### 定义于 \{#defined-in\}

[values/validators.ts:47](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L47)

***

### isConvexValidator \{#isconvexvalidator\}

• `Readonly` **isConvexValidator**: `true`

始终为 `true`。

#### 继承自 \{#inherited-from\}

BaseValidator.isConvexValidator

#### 定义于 \{#defined-in\}

[values/validators.ts:52](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L52)

***

### kind \{#kind\}

• `Readonly` **kind**: `"boolean"`

验证器的类型，`"boolean"`。

#### 定义于 \{#defined-in\}

[values/validators.ts:168](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L168)