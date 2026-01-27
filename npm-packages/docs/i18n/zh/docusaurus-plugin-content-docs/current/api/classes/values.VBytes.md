---
id: "values.VBytes"
title: "类：VBytes<Type, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VBytes

`v.bytes()` 验证器的类型。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `ArrayBuffer` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## 继承关系 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VBytes`**

## 构造函数 \{#constructors\}

### 构造函数 \{#constructor\}

• **new VBytes**&lt;`Type`, `IsOptional`&gt;(`«destructured»`)

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `ArrayBuffer` |
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

仅用于 TypeScript，表示此验证器所验证的 JS 值的 TS 类型。

#### 继承自 \{#inherited-from\}

BaseValidator.type

#### 定义于 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

仅用于 TypeScript，如果这是一个 Object 校验器，那么
这里是其属性名的 TS 类型。

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

始终为“true”。

#### 继承自 \{#inherited-from\}

BaseValidator.isConvexValidator

#### 定义于 \{#defined-in\}

[values/validators.ts:52](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L52)

***

### kind \{#kind\}

• `Readonly` **kind**: `"bytes"`

验证器的种类为 `"bytes"`。

#### 定义于 \{#defined-in\}

[values/validators.ts:192](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L192)