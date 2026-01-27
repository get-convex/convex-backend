---
id: "values.VObject"
title: "类：VObject<Type, Fields, IsOptional, FieldPaths>"
custom_edit_url: null
---

[values](../modules/values.md).VObject

`v.object()` 验证器的类型。

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `Type` |
| `Fields` | extends `Record`&lt;`string`, [`GenericValidator`](../modules/values.md#genericvalidator)&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = &#123; [Property in keyof Fields]: JoinFieldPaths&lt;Property &amp; string, Fields[Property][&quot;fieldPaths&quot;]&gt; | Property &#125;[keyof `Fields`] &amp; `string` |

## 继承层次 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`, `FieldPaths`&gt;

  ↳ **`VObject`**

## 构造函数 \{#constructors\}

### constructor \{#constructor\}

• **new VObject**&lt;`Type`, `Fields`, `IsOptional`, `FieldPaths`&gt;(`«destructured»`)

通常你会使用 `v.object({ ... })` 来代替。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `Type` |
| `Fields` | extends `Record`&lt;`string`, [`GenericValidator`](../modules/values.md#genericvalidator)&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Fields[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof `Fields`] &amp; `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |
| › `fields` | `Fields` |

#### 重写 \{#overrides\}

BaseValidator&lt;Type, IsOptional, FieldPaths&gt;.constructor

#### 定义于 \{#defined-in\}

[values/validators.ts:304](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L304)

## 属性 \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

仅适用于 TypeScript，即该验证器所校验 JS 值的 TS 类型。

#### 继承自 \{#inherited-from\}

BaseValidator.type

#### 定义于 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `FieldPaths`

仅适用于 TypeScript，如果这是一个对象验证器，那么
这就是由其各属性名组成的 TS 类型。

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

### fields \{#fields\}

• `Readonly` **fields**: `Fields`

为每个属性提供验证器的对象。

#### 定义于 \{#defined-in\}

[values/validators.ts:294](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L294)

***

### kind \{#kind\}

• `Readonly` **kind**: `"object"`

验证器的类型，`"object"`。

#### 定义于 \{#defined-in\}

[values/validators.ts:299](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L299)

## 方法 \{#methods\}

### omit \{#omit\}

▸ **omit**&lt;`K`&gt;(`...fields`): [`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Type`, `K`&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Fields`, `K`&gt;&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Omit&lt;Fields, K&gt;&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Fields`, `K`&gt;&gt;] &amp; `string`&gt;

创建一个新的 VObject，省去指定的字段。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `K` | 继承自 `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `...fields` | `K`[] | 要从此 VObject 中省略的字段名称。 |

#### 返回 \{#returns\}

[`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Type`, `K`&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Fields`, `K`&gt;&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Omit&lt;Fields, K&gt;&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Fields`, `K`&gt;&gt;] &amp; `string`&gt;

#### 定义于 \{#defined-in\}

[values/validators.ts:349](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L349)

***

### pick \{#pick\}

▸ **pick**&lt;`K`&gt;(`...fields`): [`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Type`, `Extract`&lt;keyof `Type`, `K`&gt;&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Fields`, `K`&gt;&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Pick&lt;Fields, K&gt;&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Fields`, `K`&gt;&gt;] &amp; `string`&gt;

创建一个只包含指定字段的新 VObject 实例。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `K` | 继承自 `string` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `...fields` | `K`[] | 要从该 VObject 中选取的字段名。 |

#### 返回 \{#returns\}

[`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Type`, `Extract`&lt;keyof `Type`, `K`&gt;&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Fields`, `K`&gt;&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Pick&lt;Fields, K&gt;&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Fields`, `K`&gt;&gt;] &amp; `string`&gt;

#### 定义于 \{#defined-in\}

[values/validators.ts:366](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L366)

***

### partial \{#partial\}

▸ **partial**(): [`VObject`](values.VObject.md)&lt;&#123; [K in string | number | symbol]?: Type[K] &#125;, &#123; [K in string | number | symbol]: VOptional&lt;Fields[K]&gt; &#125;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$\{Property & string}.$\{\{ [K in string \| number \| symbol]: VOptional\<Fields[K]\> }[Property]["fieldPaths"]}\` &#125;[keyof `Fields`] &amp; `string`&gt;

创建一个新的 VObject，并将其所有字段标记为可选。

#### 返回 \{#returns\}

[`VObject`](values.VObject.md)&lt;&#123; [K in string | number | symbol]?: Type[K] &#125;, &#123; [K in string | number | symbol]: VOptional&lt;Fields[K]&gt; &#125;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;&#123; [K in string | number | symbol]: VOptional&lt;Fields[K]&gt; &#125;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof `Fields`] &amp; `string`&gt;

#### 定义于 \{#defined-in\}

[values/validators.ts:386](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L386)

***

### extend \{#extend\}

▸ **extend**&lt;`NewFields`&gt;(`fields`): [`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Type` &amp; [`ObjectType`](../modules/values.md#objecttype)&lt;`NewFields`&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Fields` &amp; `NewFields`&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Fields &amp; NewFields&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Fields` &amp; `NewFields`&gt;] &amp; `string`&gt;

创建一个将额外字段合并进来的新 VObject。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `NewFields` | extends `Record`&lt;`string`, [`GenericValidator`](../modules/values.md#genericvalidator)&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `fields` | `NewFields` | 一个包含要合并到此 VObject 中的额外验证器的对象。 |

#### 返回值 \{#returns\}

[`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Type` &amp; [`ObjectType`](../modules/values.md#objecttype)&lt;`NewFields`&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Fields` &amp; `NewFields`&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Fields &amp; NewFields&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Fields` &amp; `NewFields`&gt;] &amp; `string`&gt;

#### 定义在 \{#defined-in\}

[values/validators.ts:407](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L407)