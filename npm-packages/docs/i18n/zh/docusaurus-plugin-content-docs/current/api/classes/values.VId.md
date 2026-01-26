---
id: "values.VId"
title: "类：VId<Type, IsOptional>"
custom_edit_url: null
---

[values](../modules/values.md).VId

`v.id(tableName)` 验证器的类型。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `Type` |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |

## 继承层次结构 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`&gt;

  ↳ **`VId`**

## 构造函数 \{#constructors\}

### constructor \{#constructor\}

• **new VId**&lt;`Type`, `IsOptional`&gt;(`«destructured»`)

通常你会改用 `v.id(tableName)` 来实现这一点。

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
| › `tableName` | `TableNameFromType`&lt;`Type`&gt; |

#### 重写自 \{#overrides\}

BaseValidator&lt;Type, IsOptional&gt;.constructor

#### 定义于 \{#defined-in\}

[values/validators.ts:84](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L84)

## 属性 \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

仅适用于 TypeScript，表示由此验证器校验的 JS 值在 TS 中的类型。

#### 继承自 \{#inherited-from\}

BaseValidator.type

#### 定义于 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `never`

仅适用于 TypeScript，如果这是一个 Object 验证器，则此字段的类型为其属性名称的 TS 类型。

#### 继承自 \{#inherited-from\}

BaseValidator.fieldPaths

#### 定义于 \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

表示该对象属性值验证器是否为可选。

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

### tableName \{#tablename\}

• `Readonly` **tableName**: `TableNameFromType`&lt;`Type`&gt;

经验证的 Id 所属的表名。

#### 定义在 \{#defined-in\}

[values/validators.ts:74](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L74)

***

### kind \{#kind\}

• `Readonly` **kind**: `"id"`

验证器的类型为 `"id"`。

#### 定义于 \{#defined-in\}

[values/validators.ts:79](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L79)