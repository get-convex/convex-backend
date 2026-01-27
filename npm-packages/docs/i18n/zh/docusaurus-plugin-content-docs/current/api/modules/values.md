---
id: "values"
title: "模块：values"
custom_edit_url: null
---

用于处理存储在 Convex 中的值的工具。

完整的支持类型列表见
[类型](https://docs.convex.dev/using/types)。

## 命名空间 \{#namespaces\}

* [Base64](../namespaces/values.Base64.md)

## 类 \{#classes\}

* [ConvexError](../classes/values.ConvexError.md)
* [VId](../classes/values.VId.md)
* [VFloat64](../classes/values.VFloat64.md)
* [VInt64](../classes/values.VInt64.md)
* [VBoolean](../classes/values.VBoolean.md)
* [VBytes](../classes/values.VBytes.md)
* [VString](../classes/values.VString.md)
* [VNull](../classes/values.VNull.md)
* [VAny](../classes/values.VAny.md)
* [VObject](../classes/values.VObject.md)
* [VLiteral](../classes/values.VLiteral.md)
* [VArray](../classes/values.VArray.md)
* [VRecord](../classes/values.VRecord.md)
* [VUnion](../classes/values.VUnion.md)

## 类型别名 \{#type-aliases\}

### GenericValidator \{#genericvalidator\}

Ƭ **GenericValidator**: [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt;

所有验证器都必须从中扩展的类型。

#### 定义于 \{#defined-in\}

[values/validator.ts:27](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L27)

***

### AsObjectValidator \{#asobjectvalidator\}

Ƭ **AsObjectValidator**&lt;`V`&gt;: `V` extends [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; ? `V` : `V` extends [`PropertyValidators`](values.md#propertyvalidators) ? [`Validator`](values.md#validator)&lt;[`ObjectType`](values.md#objecttype)&lt;`V`&gt;&gt; : `never`

将一个以验证器作为属性的对象转换为验证器。
如果传入的是验证器，则直接返回。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `V` | 受限于 [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |

#### 定义于 \{#defined-in\}

[values/validator.ts:61](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L61)

***

### PropertyValidators \{#propertyvalidators\}

Ƭ **PropertyValidators**: `Record`&lt;`string`, [`Validator`](values.md#validator)&lt;`any`, [`OptionalProperty`](values.md#optionalproperty), `any`&gt;&gt;

用于对象中每个属性的验证器。

它表示为一个对象，该对象将属性名映射到其对应的
[Validator](values.md#validator)。

#### 定义于 \{#defined-in\}

[values/validator.ts:242](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L242)

***

### ObjectType \{#objecttype\}

Ƭ **ObjectType**&lt;`Fields`&gt;: [`Expand`](server.md#expand)&lt;&#123; [Property in OptionalKeys&lt;Fields&gt;]?: Exclude&lt;Infer&lt;Fields[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in RequiredKeys&lt;Fields&gt;]: Infer&lt;Fields[Property]&gt; &#125;&gt;

从 [PropertyValidators](values.md#propertyvalidators) 推导出对象的类型。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Fields` | 受限于 [`PropertyValidators`](values.md#propertyvalidators) |

#### 定义于 \{#defined-in\}

[values/validator.ts:252](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L252)

***

### Infer \{#infer\}

Ƭ **Infer**&lt;`T`&gt;: `T`[`"type"`]

从 validator 中推断出 TypeScript 类型。

示例用法：

```ts
const objectSchema = v.object({
  property: v.string(),
});
type MyObject = Infer<typeof objectSchema>; // { property: string }
```

**`Type Param`**

由 [v](values.md#v) 构造的 [Validator](values.md#validator) 的类型。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | extends [`Validator`](values.md#validator)&lt;`any`, [`OptionalProperty`](values.md#optionalproperty), `any`&gt; |

#### 定义于 \{#defined-in\}

[values/validator.ts:294](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L294)

***

### VOptional \{#voptional\}

Ƭ **VOptional**&lt;`T`&gt;: `T` extends [`VId`](../classes/values.VId.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VId`](../classes/values.VId.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VString`](../classes/values.VString.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VString`](../classes/values.VString.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VFloat64`](../classes/values.VFloat64.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VFloat64`](../classes/values.VFloat64.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VInt64`](../classes/values.VInt64.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VInt64`](../classes/values.VInt64.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VBoolean`](../classes/values.VBoolean.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VBoolean`](../classes/values.VBoolean.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VNull`](../classes/values.VNull.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VNull`](../classes/values.VNull.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VAny`](../classes/values.VAny.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VAny`](../classes/values.VAny.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VLiteral`](../classes/values.VLiteral.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VLiteral`](../classes/values.VLiteral.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VBytes`](../classes/values.VBytes.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VBytes`](../classes/values.VBytes.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VObject`](../classes/values.VObject.md)&lt;infer Type, infer Fields, [`OptionalProperty`](values.md#optionalproperty), infer FieldPaths&gt; ? [`VObject`](../classes/values.VObject.md)&lt;`Type` | `undefined`, `Fields`, `"optional"`, `FieldPaths`&gt; : `T` extends [`VArray`](../classes/values.VArray.md)&lt;infer Type, infer Element, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VArray`](../classes/values.VArray.md)&lt;`Type` | `undefined`, `Element`, `"optional"`&gt; : `T` extends [`VRecord`](../classes/values.VRecord.md)&lt;infer Type, infer Key, infer Value, [`OptionalProperty`](values.md#optionalproperty), infer FieldPaths&gt; ? [`VRecord`](../classes/values.VRecord.md)&lt;`Type` | `undefined`, `Key`, `Value`, `"optional"`, `FieldPaths`&gt; : `T` extends [`VUnion`](../classes/values.VUnion.md)&lt;infer Type, infer Members, [`OptionalProperty`](values.md#optionalproperty), infer FieldPaths&gt; ? [`VUnion`](../classes/values.VUnion.md)&lt;`Type` | `undefined`, `Members`, `"optional"`, `FieldPaths`&gt; : `never`

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | extends [`Validator`](values.md#validator)&lt;`any`, [`OptionalProperty`](values.md#optionalproperty), `any`&gt; |

#### 定义于 \{#defined-in\}

[values/validators.ts:648](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L648)

***

### OptionalProperty \{#optionalproperty\}

Ƭ **OptionalProperty**: `"optional"` | `"required"`

表示对象中某个属性是可选还是必填的类型。

#### 定义于 \{#defined-in\}

[values/validators.ts:681](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L681)

***

### Validator \{#validator\}

Ƭ **Validator**&lt;`Type`, `IsOptional`, `FieldPaths`&gt;: [`VId`](../classes/values.VId.md)&lt;`Type`, `IsOptional`&gt; | [`VString`](../classes/values.VString.md)&lt;`Type`, `IsOptional`&gt; | [`VFloat64`](../classes/values.VFloat64.md)&lt;`Type`, `IsOptional`&gt; | [`VInt64`](../classes/values.VInt64.md)&lt;`Type`, `IsOptional`&gt; | [`VBoolean`](../classes/values.VBoolean.md)&lt;`Type`, `IsOptional`&gt; | [`VNull`](../classes/values.VNull.md)&lt;`Type`, `IsOptional`&gt; | [`VAny`](../classes/values.VAny.md)&lt;`Type`, `IsOptional`&gt; | [`VLiteral`](../classes/values.VLiteral.md)&lt;`Type`, `IsOptional`&gt; | [`VBytes`](../classes/values.VBytes.md)&lt;`Type`, `IsOptional`&gt; | [`VObject`](../classes/values.VObject.md)&lt;`Type`, `Record`&lt;`string`, [`Validator`](values.md#validator)&lt;`any`, [`OptionalProperty`](values.md#optionalproperty), `any`&gt;&gt;, `IsOptional`, `FieldPaths`&gt; | [`VArray`](../classes/values.VArray.md)&lt;`Type`, [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt;, `IsOptional`&gt; | [`VRecord`](../classes/values.VRecord.md)&lt;`Type`, [`Validator`](values.md#validator)&lt;`string`, `"required"`, `any`&gt;, [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt;, `IsOptional`, `FieldPaths`&gt; | [`VUnion`](../classes/values.VUnion.md)&lt;`Type`, [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt;[], `IsOptional`, `FieldPaths`&gt;

用于 Convex 值的验证器。

应使用验证器构建器 [v](values.md#v) 来构造。

一个验证器封装了：

* 此值的 TypeScript 类型。
* 如果该字段包含在对象中，此字段是否应为可选。
* 可用于在该值上构建索引的索引字段路径集合的 TypeScript 类型。
* 验证器的 JSON 表示形式。

某些具体类型的验证器包含额外的信息：例如
`ArrayValidator` 包含一个 `element` 属性，其中包含用于验证列表中每个元素的验证器。使用共享的 `kind` 属性
来标识验证器的类型。

未来版本中可能会添加更多验证器，因此对验证器 `kind` 进行穷举的
switch 语句在 Convex 的未来版本中可能会失效。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | `Type` |
| `IsOptional` | extends [`OptionalProperty`](values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `never` |

#### 定义于 \{#defined-in\}

[values/validators.ts:706](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L706)

***

### ObjectFieldType \{#objectfieldtype\}

Ƭ **ObjectFieldType**: `Object`

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 |
| :------ | :------ |
| `fieldType` | [`ValidatorJSON`](values.md#validatorjson) |
| `optional` | `boolean` |

#### 定义于 \{#defined-in\}

[values/validators.ts:747](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L747)

***

### ValidatorJSON \{#validatorjson\}

Ƭ **ValidatorJSON**: &#123; `type`: `"null"`  &#125; | &#123; `type`: `"number"`  &#125; | &#123; `type`: `"bigint"`  &#125; | &#123; `type`: `"boolean"`  &#125; | &#123; `type`: `"string"`  &#125; | &#123; `type`: `"bytes"`  &#125; | &#123; `type`: `"any"`  &#125; | &#123; `type`: `"literal"` ; `value`: [`JSONValue`](values.md#jsonvalue)  &#125; | &#123; `type`: `"id"` ; `tableName`: `string`  &#125; | &#123; `type`: `"array"` ; `value`: [`ValidatorJSON`](values.md#validatorjson)  &#125; | &#123; `type`: `"record"` ; `keys`: [`RecordKeyValidatorJSON`](values.md#recordkeyvalidatorjson) ; `values`: [`RecordValueValidatorJSON`](values.md#recordvaluevalidatorjson)  &#125; | &#123; `type`: `"object"` ; `value`: `Record`&lt;`string`, [`ObjectFieldType`](values.md#objectfieldtype)&gt;  &#125; | &#123; `type`: `"union"` ; `value`: [`ValidatorJSON`](values.md#validatorjson)[]  &#125;

#### 定义于 \{#defined-in\}

[values/validators.ts:749](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L749)

***

### RecordKeyValidatorJSON \{#recordkeyvalidatorjson\}

Ƭ **RecordKeyValidatorJSON**: &#123; `type`: `"string"`  &#125; | &#123; `type`: `"id"` ; `tableName`: `string`  &#125; | &#123; `type`: `"union"` ; `value`: [`RecordKeyValidatorJSON`](values.md#recordkeyvalidatorjson)[]  &#125;

#### 定义于 \{#defined-in\}

[values/validators.ts:768](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L768)

***

### RecordValueValidatorJSON \{#recordvaluevalidatorjson\}

Ƭ **RecordValueValidatorJSON**: [`ObjectFieldType`](values.md#objectfieldtype) &amp; &#123; `optional`: `false`  &#125;

#### 定义于 \{#defined-in\}

[values/validators.ts:773](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L773)

***

### JSONValue \{#jsonvalue\}

Ƭ **JSONValue**: `null` | `boolean` | `number` | `string` | [`JSONValue`](values.md#jsonvalue)[] | &#123; `[key: string]`: [`JSONValue`](values.md#jsonvalue);  &#125;

JavaScript 中可序列化为 JSON 的值的类型。

#### 定义于 \{#defined-in\}

[values/value.ts:24](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L24)

***

### GenericId \{#genericid\}

Ƭ **GenericId**&lt;`TableName`&gt;: `string` &amp; &#123; `__tableName`: `TableName`  &#125;

Convex 文档的标识符。

Convex 文档通过其 `Id` 唯一标识，该值可以通过 `_id` 字段访问。要了解更多信息，请参阅 [Document IDs](https://docs.convex.dev/database/document-ids)。

可以在查询和变更函数中使用 `db.get(tableName, id)` 来加载文档。

ID 是经过 base32 编码的字符串，并且可以安全地用于 URL。

ID 在运行时只是字符串，但此类型可用于在编译时将其与其他字符串区分开来。

如果你使用代码生成，请在 `convex/_generated/dataModel.d.ts` 中使用为你的数据模型生成的 `Id` 类型。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `TableName` | extends `string` | 表名的字符串字面量类型（如 &quot;users&quot;）。 |

#### 定义于 \{#defined-in\}

[values/value.ts:52](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L52)

***

### Value \{#value\}

Ƭ **Value**: `null` | `bigint` | `number` | `boolean` | `string` | `ArrayBuffer` | [`Value`](values.md#value)[] | &#123; `[key: string]`: `undefined` | [`Value`](values.md#value);  &#125;

Convex 支持的值类型。

值可以：

* 存储在文档中。
* 作为查询和变更函数的参数和返回类型使用。

你可以在
[Types](https://docs.convex.dev/using/types)
中查看完整的受支持类型列表。

#### 定义于 \{#defined-in\}

[values/value.ts:66](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L66)

***

### NumericValue \{#numericvalue\}

Ƭ **NumericValue**: `bigint` | `number`

可以用来表示数值的 [值](values.md#value) 类型。

#### 定义于 \{#defined-in\}

[values/value.ts:81](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L81)

## 变量 \{#variables\}

### v \{#v\}

• `Const` **v**: `Object`

验证器构建器。

此构建器允许你构建用于 Convex 值的验证器。

验证器可用于[模式定义](https://docs.convex.dev/database/schemas)，
也可作为 Convex 函数的输入验证器。

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 |
| :------ | :------ |
| `id` | &lt;TableName&gt;(`tableName`: `TableName`) =&gt; [`VId`](../classes/values.VId.md)&lt;[`GenericId`](values.md#genericid)&lt;`TableName`&gt;, `"required"`&gt; |
| `null` | () =&gt; [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt; |
| `number` | () =&gt; [`VFloat64`](../classes/values.VFloat64.md)&lt;`number`, `"required"`&gt; |
| `float64` | () =&gt; [`VFloat64`](../classes/values.VFloat64.md)&lt;`number`, `"required"`&gt; |
| `bigint` | () =&gt; [`VInt64`](../classes/values.VInt64.md)&lt;`bigint`, `"required"`&gt; |
| `int64` | () =&gt; [`VInt64`](../classes/values.VInt64.md)&lt;`bigint`, `"required"`&gt; |
| `boolean` | () =&gt; [`VBoolean`](../classes/values.VBoolean.md)&lt;`boolean`, `"required"`&gt; |
| `string` | () =&gt; [`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt; |
| `bytes` | () =&gt; [`VBytes`](../classes/values.VBytes.md)&lt;`ArrayBuffer`, `"required"`&gt; |
| `literal` | &lt;T&gt;(`literal`: `T`) =&gt; [`VLiteral`](../classes/values.VLiteral.md)&lt;`T`, `"required"`&gt; |
| `array` | &lt;T&gt;(`element`: `T`) =&gt; [`VArray`](../classes/values.VArray.md)&lt;`T`[`"type"`][], `T`, `"required"`&gt; |
| `object` | &lt;T&gt;(`fields`: `T`) =&gt; [`VObject`](../classes/values.VObject.md)&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;T[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;T[Property]&gt; &#125;&gt;, `T`, `"required"`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;T[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof `T`] &amp; `string`&gt; |
| `record` | &lt;Key, Value&gt;(`keys`: `Key`, `values`: `Value`) =&gt; [`VRecord`](../classes/values.VRecord.md)&lt;`Record`&lt;[`Infer`](values.md#infer)&lt;`Key`&gt;, `Value`[`"type"`]&gt;, `Key`, `Value`, `"required"`, `string`&gt; |
| `union` | &lt;T&gt;(...`members`: `T`) =&gt; [`VUnion`](../classes/values.VUnion.md)&lt;`T`[`number`][`"type"`], `T`, `"required"`, `T`[`number`][`"fieldPaths"`]&gt; |
| `any` | () =&gt; [`VAny`](../classes/values.VAny.md)&lt;`any`, `"required"`, `string`&gt; |
| `optional` | &lt;T&gt;(`value`: `T`) =&gt; [`VOptional`](values.md#voptional)&lt;`T`&gt; |
| `nullable` | &lt;T&gt;(`value`: `T`) =&gt; [`VUnion`](../classes/values.VUnion.md)&lt;`T` | [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;[`"type"`], [`T`, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"required"`, `T` | [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;[`"fieldPaths"`]&gt; |

#### 定义于 \{#defined-in\}

[values/validator.ts:80](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L80)

## 函数 \{#functions\}

### compareValues \{#comparevalues\}

▸ **compareValues**(`k1`, `k2`): `number`

#### 参数 \{#parameters\}

| 参数名 | 类型 |
| :------ | :------ |
| `k1` | `undefined` | [`Value`](values.md#value) |
| `k2` | `undefined` | [`Value`](values.md#value) |

#### 返回值 \{#returns\}

`number`

#### 定义于 \{#defined-in\}

[values/compare.ts:4](https://github.com/get-convex/convex-js/blob/main/src/values/compare.ts#L4)

***

### asObjectValidator \{#asobjectvalidator\}

▸ **asObjectValidator**&lt;`V`&gt;(`obj`): `V` extends [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; ? `V` : `V` extends [`PropertyValidators`](values.md#propertyvalidators) ? [`Validator`](values.md#validator)&lt;[`ObjectType`](values.md#objecttype)&lt;`V`&gt;&gt; : `never`

将一个以验证器作为属性的对象转换为一个验证器。
如果传入的是验证器，则直接返回该验证器。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `V` | extends [`PropertyValidators`](values.md#propertyvalidators) | [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; |

#### 参数 \{#parameters\}

| 参数名 | 类型 |
| :------ | :------ |
| `obj` | `V` |

#### 返回值 \{#returns\}

`V` extends [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; ? `V` : `V` extends [`PropertyValidators`](values.md#propertyvalidators) ? [`Validator`](values.md#validator)&lt;[`ObjectType`](values.md#objecttype)&lt;`V`&gt;&gt; : `never`

#### 定义于 \{#defined-in\}

[values/validator.ts:39](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L39)

***

### jsonToConvex \{#jsontoconvex\}

▸ **jsonToConvex**(`value`): [`Value`](values.md#value)

从其 JSON 表示中解析出一个 Convex 值。

此函数会将序列化的 Int64 反序列化为 `BigInt`，将 Bytes 反序列化为 `ArrayBuffer` 等。

要了解更多关于 Convex 值的信息，请参阅 [Types](https://docs.convex.dev/using/types)。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `value` | [`JSONValue`](values.md#jsonvalue) | 先前使用 [convexToJson](values.md#convextojson) 创建的 Convex 值的 JSON 表示。 |

#### 返回值 \{#returns\}

[`Value`](values.md#value)

JavaScript 中对 Convex 值的表示。

#### 定义于 \{#defined-in\}

[values/value.ts:187](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L187)

***

### convexToJson \{#convextojson\}

▸ **convexToJson**(`value`): [`JSONValue`](values.md#jsonvalue)

将 Convex 值转换为其对应的 JSON 表示形式。

使用 [jsonToConvex](values.md#jsontoconvex) 来恢复原始值。

要进一步了解 Convex 值，请参阅 [类型](https://docs.convex.dev/using/types)。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `value` | [`Value`](values.md#value) | 要转换为 JSON 的 Convex 值。 |

#### 返回值 \{#returns\}

[`JSONValue`](values.md#jsonvalue)

`value` 的 JSON 表示。

#### 定义于 \{#defined-in\}

[values/value.ts:429](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L429)