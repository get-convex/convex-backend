---
id: "values.VLiteral"
title: "类：VLiteral<Type, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VLiteral

`v.literal()` 验证器的类型。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `Type` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## 继承关系 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VLiteral`**

## 构造函数 \{#constructors\}

### constructor \{#constructor\}

• **new VLiteral**&lt;`Type`, `IsOptional`&gt;(`«destructured»`)

通常你会使用 `v.literal(value)` 来代替。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `Type` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |
| › `value` | `Type` |

#### 重写自 \{#overrides\}

BaseValidator&lt;Type, IsOptional&gt;.constructor

#### 定义于 \{#defined-in\}

[values/validators.ts:441](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L441)

## 属性 \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

仅用于 TypeScript，表示该校验器所校验的 JS 值对应的 TS 类型。

#### 继承自 \{#inherited-from\}

BaseValidator.type

#### 定义于 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

仅适用于 TypeScript，如果这是一个对象类型的校验器，则
这里表示其属性名的 TS 类型。

#### 继承自 \{#inherited-from\}

BaseValidator.fieldPaths

#### 定义于 \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

指示此对象属性值验证器是否为可选。

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

### value \{#value\}

• `Readonly` **value**: `Type`

经过验证的值必须与之相等的值。

#### 定义于 \{#defined-in\}

[values/validators.ts:431](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L431)

***

### kind \{#kind\}

• `Readonly` **kind**: `"literal"`

验证器的类别，即 `"literal"`。

#### 定义于 \{#defined-in\}

[values/validators.ts:436](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L436)