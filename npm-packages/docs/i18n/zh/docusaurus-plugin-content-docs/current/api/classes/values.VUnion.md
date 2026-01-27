---
id: "values.VUnion"
title: "类：VUnion<Type, T, IsOptional, FieldPaths>"
custom_edit_url: null
---

[values](../modules/values.md).VUnion

`v.union()` 验证器的类型。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `Type` |
| `T` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt;[] |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `T`[`number`][`"fieldPaths"`] |

## 继承层级 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`, `FieldPaths`&gt;

  ↳ **`VUnion`**

## 构造函数 \{#constructors\}

### constructor \{#constructor\}

• **new VUnion**&lt;`Type`, `T`, `IsOptional`, `FieldPaths`&gt;(`«destructured»`)

通常你会使用 `v.union(...members)` 来代替。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `Type` |
| `T` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt;[] |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `T`[`number`][`"fieldPaths"`] |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |
| › `members` | `T` |

#### 重写 \{#overrides\}

BaseValidator&lt;Type, IsOptional, FieldPaths&gt;.constructor

#### 定义于 \{#defined-in\}

[values/validators.ts:619](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L619)

## 属性 \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

仅用于 TypeScript，表示此验证器所验证的 JS 值对应的 TS 类型。

#### 继承自 \{#inherited-from\}

BaseValidator.type

#### 定义于 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `FieldPaths`

仅适用于 TypeScript，如果这是一个 Object 校验器，则这是其属性名的 TS 类型。

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

### members \{#members\}

• `Readonly` **members**: `T`

验证器数组，其中至少有一个必须能匹配该值。

#### 定义于 \{#defined-in\}

[values/validators.ts:609](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L609)

***

### kind \{#kind\}

• `Readonly` **kind**: `"union"`

验证器的种类，`"union"`。

#### 定义于 \{#defined-in\}

[values/validators.ts:614](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L614)