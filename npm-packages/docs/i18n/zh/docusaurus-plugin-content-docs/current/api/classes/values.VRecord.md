---
id: "values.VRecord"
title: "类：VRecord<Type, Key, Value, IsOptional, FieldPaths>"
custom_edit_url: null
---

[values](../modules/values.md).VRecord

`v.record()` 验证器的类型。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `Type` |
| `Key` | extends [`Validator`](../modules/values.md#validator)&lt;`string`, `"required"`, `any`&gt; |
| `Value` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `string` |

## 继承层次结构 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`, `FieldPaths`&gt;

  ↳ **`VRecord`**

## 构造函数 \{#constructors\}

### constructor \{#constructor\}

• **new VRecord**&lt;`Type`, `Key`, `Value`, `IsOptional`, `FieldPaths`&gt;(`«destructured»`)

通常你会使用 `v.record(key, value)` 来代替。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `Type` |
| `Key` | extends [`Validator`](../modules/values.md#validator)&lt;`string`, `"required"`, `any`&gt; |
| `Value` | extends [`Validator`](../modules/values.md#validator)&lt;`any`, `"required"`, `any`&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |
| › `key` | `Key` |
| › `value` | `Value` |

#### 重写 \{#overrides\}

BaseValidator&lt;Type, IsOptional, FieldPaths&gt;.constructor

#### 定义于 \{#defined-in\}

[values/validators.ts:547](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L547)

## 属性 \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

仅适用于 TypeScript，表示此验证器所校验 JS 值的 TS 类型。

#### 继承自 \{#inherited-from\}

BaseValidator.type

#### 定义于 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `FieldPaths`

仅用于 TypeScript，如果这是一个 Object 校验器，则它是该对象属性名的 TS 类型。

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

始终是 `"true"`。

#### 继承自 \{#inherited-from\}

BaseValidator.isConvexValidator

#### 定义在 \{#defined-in\}

[values/validators.ts:52](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L52)

***

### key \{#key\}

• `Readonly` **key**: `Key`

用于验证此 record 的键。

#### 定义于 \{#defined-in\}

[values/validators.ts:532](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L532)

***

### value \{#value\}

• `Readonly` **value**: `Value`

用于验证该 record 各字段值的验证器。

#### 定义于 \{#defined-in\}

[values/validators.ts:537](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L537)

***

### kind \{#kind\}

• `Readonly` **kind**: `"record"`

此验证器的种类为 `"record"`。

#### 定义于 \{#defined-in\}

[values/validators.ts:542](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L542)