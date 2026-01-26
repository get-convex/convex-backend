---
id: "values.VArray"
title: "类：VArray<Type, Element, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VArray

`v.array()` 验证器的类型。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `Type` |
| `Element` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## 继承层级 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VArray`**

## 构造函数 \{#constructors\}

### constructor \{#constructor\}

• **new VArray**&lt;`Type`, `Element`, `IsOptional`&gt;(`«destructured»`)

通常应改为使用 `v.array(element)`。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `Type` |
| `Element` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |
| › `element` | `Element` |

#### 重写 \{#overrides\}

BaseValidator&lt;Type, IsOptional&gt;.constructor

#### 定义于 \{#defined-in\}

[values/validators.ts:490](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L490)

## 属性 \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

仅适用于 TypeScript，表示由此验证器验证的 JS 值的 TS 类型。

#### 继承自 \{#inherited-from\}

BaseValidator.type

#### 定义于 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

仅适用于 TypeScript，如果这是一个 Object 验证器，则这是其属性名的 TypeScript 类型。

#### 继承自 \{#inherited-from\}

BaseValidator.fieldPaths

#### 定义于 \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

指示该验证器是否用于可选的 Object 属性值。

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

### element \{#element\}

• `Readonly` **element**: `Element`

用于验证数组中各元素的验证器。

#### 定义于 \{#defined-in\}

[values/validators.ts:480](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L480)

***

### kind \{#kind\}

• `Readonly` **kind**: `"array"`

验证器的种类：`"array"`。

#### 定义于 \{#defined-in\}

[values/validators.ts:485](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L485)