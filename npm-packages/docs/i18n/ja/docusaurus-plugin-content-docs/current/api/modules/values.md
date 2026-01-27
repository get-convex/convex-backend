---
id: "values"
title: "モジュール: values"
custom_edit_url: null
---

Convex に保存されている値を操作するためのユーティリティです。

サポートされている型の完全な一覧は
[型](https://docs.convex.dev/using/types)
を参照してください。

## 名前空間 \{#namespaces\}

* [Base64](../namespaces/values.Base64.md)

## クラス \{#classes\}

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

## 型エイリアス \{#type-aliases\}

### GenericValidator \{#genericvalidator\}

Ƭ **GenericValidator**: [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt;

すべてのバリデータが拡張しなければならない型です。

#### 定義元 \{#defined-in\}

[values/validator.ts:27](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L27)

***

### AsObjectValidator \{#asobjectvalidator\}

Ƭ **AsObjectValidator**&lt;`V`&gt;: `V` extends [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; ? `V` : `V` extends [`PropertyValidators`](values.md#propertyvalidators) ? [`Validator`](values.md#validator)&lt;[`ObjectType`](values.md#objecttype)&lt;`V`&gt;&gt; : `never`

プロパティとして validator を持つオブジェクトを、1つの validator に変換します。
すでに validator が渡された場合は、その validator をそのまま返します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `V` | extends [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |

#### 定義元 \{#defined-in\}

[values/validator.ts:61](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L61)

***

### PropertyValidators \{#propertyvalidators\}

Ƭ **PropertyValidators**: `Record`&lt;`string`, [`Validator`](values.md#validator)&lt;`any`, [`OptionalProperty`](values.md#optionalproperty), `any`&gt;&gt;

オブジェクトの各プロパティ用のバリデーター。

これは、プロパティ名から対応する
[Validator](values.md#validator)
へのマッピングを表すオブジェクトとして表現されます。

#### 定義元 \{#defined-in\}

[values/validator.ts:242](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L242)

***

### ObjectType \{#objecttype\}

Ƭ **ObjectType**&lt;`Fields`&gt;: [`Expand`](server.md#expand)&lt;&#123; [Property in OptionalKeys&lt;Fields&gt;]?: Exclude&lt;Infer&lt;Fields[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in RequiredKeys&lt;Fields&gt;]: Infer&lt;Fields[Property]&gt; &#125;&gt;

[PropertyValidators](values.md#propertyvalidators) からオブジェクト型を算出します。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Fields` | extends [`PropertyValidators`](values.md#propertyvalidators) |

#### 定義元 \{#defined-in\}

[values/validator.ts:252](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L252)

***

### Infer \{#infer\}

Ƭ **Infer**&lt;`T`&gt;: `T`[`"type"`]

バリデータから対応する TypeScript 型を抽出します。

使用例:

```ts
const objectSchema = v.object({
  property: v.string(),
});
type MyObject = Infer<typeof objectSchema>; // { property: string }
```

**`Type Param`**

[v](values.md#v) を使って構築した [Validator](values.md#validator) の型。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `T` | extends [`Validator`](values.md#validator)&lt;`any`, [`OptionalProperty`](values.md#optionalproperty), `any`&gt; |

#### 定義場所 \{#defined-in\}

[values/validator.ts:294](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L294)

***

### VOptional \{#voptional\}

Ƭ **VOptional**&lt;`T`&gt;: `T` extends [`VId`](../classes/values.VId.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VId`](../classes/values.VId.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VString`](../classes/values.VString.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VString`](../classes/values.VString.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VFloat64`](../classes/values.VFloat64.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VFloat64`](../classes/values.VFloat64.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VInt64`](../classes/values.VInt64.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VInt64`](../classes/values.VInt64.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VBoolean`](../classes/values.VBoolean.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VBoolean`](../classes/values.VBoolean.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VNull`](../classes/values.VNull.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VNull`](../classes/values.VNull.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VAny`](../classes/values.VAny.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VAny`](../classes/values.VAny.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VLiteral`](../classes/values.VLiteral.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VLiteral`](../classes/values.VLiteral.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VBytes`](../classes/values.VBytes.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VBytes`](../classes/values.VBytes.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VObject`](../classes/values.VObject.md)&lt;infer Type, infer Fields, [`OptionalProperty`](values.md#optionalproperty), infer FieldPaths&gt; ? [`VObject`](../classes/values.VObject.md)&lt;`Type` | `undefined`, `Fields`, `"optional"`, `FieldPaths`&gt; : `T` extends [`VArray`](../classes/values.VArray.md)&lt;infer Type, infer Element, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VArray`](../classes/values.VArray.md)&lt;`Type` | `undefined`, `Element`, `"optional"`&gt; : `T` extends [`VRecord`](../classes/values.VRecord.md)&lt;infer Type, infer Key, infer Value, [`OptionalProperty`](values.md#optionalproperty), infer FieldPaths&gt; ? [`VRecord`](../classes/values.VRecord.md)&lt;`Type` | `undefined`, `Key`, `Value`, `"optional"`, `FieldPaths`&gt; : `T` extends [`VUnion`](../classes/values.VUnion.md)&lt;infer Type, infer Members, [`OptionalProperty`](values.md#optionalproperty), infer FieldPaths&gt; ? [`VUnion`](../classes/values.VUnion.md)&lt;`Type` | `undefined`, `Members`, `"optional"`, `FieldPaths`&gt; : `never`

#### 型パラメーター \{#type-parameters\}

| 名称 | 型 |
| :------ | :------ |
| `T` | extends [`Validator`](values.md#validator)&lt;`any`, [`OptionalProperty`](values.md#optionalproperty), `any`&gt; |

#### 定義元 \{#defined-in\}

[values/validators.ts:648](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L648)

***

### OptionalProperty \{#optionalproperty\}

Ƭ **OptionalProperty**: `"optional"` | `"required"`

オブジェクトのプロパティが任意か必須かを表す型。

#### 定義元 \{#defined-in\}

[values/validators.ts:681](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L681)

***

### Validator \{#validator\}

Ƭ **Validator**&lt;`Type`, `IsOptional`, `FieldPaths`&gt;: [`VId`](../classes/values.VId.md)&lt;`Type`, `IsOptional`&gt; | [`VString`](../classes/values.VString.md)&lt;`Type`, `IsOptional`&gt; | [`VFloat64`](../classes/values.VFloat64.md)&lt;`Type`, `IsOptional`&gt; | [`VInt64`](../classes/values.VInt64.md)&lt;`Type`, `IsOptional`&gt; | [`VBoolean`](../classes/values.VBoolean.md)&lt;`Type`, `IsOptional`&gt; | [`VNull`](../classes/values.VNull.md)&lt;`Type`, `IsOptional`&gt; | [`VAny`](../classes/values.VAny.md)&lt;`Type`, `IsOptional`&gt; | [`VLiteral`](../classes/values.VLiteral.md)&lt;`Type`, `IsOptional`&gt; | [`VBytes`](../classes/values.VBytes.md)&lt;`Type`, `IsOptional`&gt; | [`VObject`](../classes/values.VObject.md)&lt;`Type`, `Record`&lt;`string`, [`Validator`](values.md#validator)&lt;`any`, [`OptionalProperty`](values.md#optionalproperty), `any`&gt;&gt;, `IsOptional`, `FieldPaths`&gt; | [`VArray`](../classes/values.VArray.md)&lt;`Type`, [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt;, `IsOptional`&gt; | [`VRecord`](../classes/values.VRecord.md)&lt;`Type`, [`Validator`](values.md#validator)&lt;`string`, `"required"`, `any`&gt;, [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt;, `IsOptional`, `FieldPaths`&gt; | [`VUnion`](../classes/values.VUnion.md)&lt;`Type`, [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt;[], `IsOptional`, `FieldPaths`&gt;

Convex の値に対するバリデータ。

これはバリデータビルダー [v](values.md#v) を使って構築します。

バリデータは次のことをカプセル化します:

* この値の TypeScript 型。
* このフィールドがオブジェクトに含まれている場合に、省略可能かどうか。
* この値に対してインデックスを構築するのに使用できるインデックスフィールドパス集合の
  TypeScript 型。
* バリデータの JSON 表現。

特定の種類のバリデータは、追加情報を含みます。たとえば
`ArrayValidator` は、リスト内の各要素を検証するために使用されるバリデータを保持する
`element` プロパティを含みます。共有の `kind` プロパティを使って
バリデータの種類を識別してください。

将来のリリースで、より多くのバリデータが追加される可能性があります。そのため、
バリデータの `kind` に対する網羅的な switch 文は、
将来の Convex のリリースで動作しなくなることを想定しておくべきです。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `Type` |
| `IsOptional` | extends [`OptionalProperty`](values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `never` |

#### 定義場所 \{#defined-in\}

[values/validators.ts:706](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L706)

***

### ObjectFieldType \{#objectfieldtype\}

Ƭ **ObjectFieldType**: `Object`

#### 型宣言 \{#type-declaration\}

| 名前 | 型 |
| :------ | :------ |
| `fieldType` | [`ValidatorJSON`](values.md#validatorjson) |
| `optional` | `boolean` |

#### 定義場所 \{#defined-in\}

[values/validators.ts:747](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L747)

***

### ValidatorJSON \{#validatorjson\}

Ƭ **ValidatorJSON**: &#123; `type`: `"null"`  &#125; | &#123; `type`: `"number"`  &#125; | &#123; `type`: `"bigint"`  &#125; | &#123; `type`: `"boolean"`  &#125; | &#123; `type`: `"string"`  &#125; | &#123; `type`: `"bytes"`  &#125; | &#123; `type`: `"any"`  &#125; | &#123; `type`: `"literal"` ; `value`: [`JSONValue`](values.md#jsonvalue)  &#125; | &#123; `type`: `"id"` ; `tableName`: `string`  &#125; | &#123; `type`: `"array"` ; `value`: [`ValidatorJSON`](values.md#validatorjson)  &#125; | &#123; `type`: `"record"` ; `keys`: [`RecordKeyValidatorJSON`](values.md#recordkeyvalidatorjson) ; `values`: [`RecordValueValidatorJSON`](values.md#recordvaluevalidatorjson)  &#125; | &#123; `type`: `"object"` ; `value`: `Record`&lt;`string`, [`ObjectFieldType`](values.md#objectfieldtype)&gt;  &#125; | &#123; `type`: `"union"` ; `value`: [`ValidatorJSON`](values.md#validatorjson)[]  &#125;

#### 定義場所 \{#defined-in\}

[values/validators.ts:749](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L749)

***

### RecordKeyValidatorJSON \{#recordkeyvalidatorjson\}

Ƭ **RecordKeyValidatorJSON**: &#123; `type`: `"string"`  &#125; | &#123; `type`: `"id"` ; `tableName`: `string`  &#125; | &#123; `type`: `"union"` ; `value`: [`RecordKeyValidatorJSON`](values.md#recordkeyvalidatorjson)[]  &#125;

#### 定義元 \{#defined-in\}

[values/validators.ts:768](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L768)

***

### RecordValueValidatorJSON \{#recordvaluevalidatorjson\}

Ƭ **RecordValueValidatorJSON**: [`ObjectFieldType`](values.md#objectfieldtype) &amp; &#123; `optional`: `false`  &#125;

#### 定義場所 \{#defined-in\}

[values/validators.ts:773](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L773)

***

### JSONValue \{#jsonvalue\}

Ƭ **JSONValue**: `null` | `boolean` | `number` | `string` | [`JSONValue`](values.md#jsonvalue)[] | &#123; `[key: string]`: [`JSONValue`](values.md#jsonvalue);  &#125;

JSON にシリアライズ可能な JavaScript 値の型です。

#### 定義元 \{#defined-in\}

[values/value.ts:24](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L24)

***

### GenericId \{#genericid\}

Ƭ **GenericId**&lt;`TableName`&gt;: `string` &amp; &#123; `__tableName`: `TableName`  &#125;

Convex 内のドキュメントを指す識別子です。

Convex のドキュメントは、一意の `Id` によって識別され、この値には `_id` フィールドからアクセスできます。詳しくは、[Document IDs](https://docs.convex.dev/database/document-ids) を参照してください。

ドキュメントは、クエリ関数およびミューテーション関数内で `db.get(tableName, id)` を使って読み込むことができます。

ID は、base32 でエンコードされた URL セーフな文字列です。

ID は実行時には単なる文字列ですが、この型を使うことでコンパイル時に他の文字列と区別できます。

コード生成を使用している場合は、`convex/_generated/dataModel.d.ts` に生成されるデータモデル用の `Id` 型を使用してください。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 | 説明 |
| :------ | :------ | :------ |
| `TableName` | extends `string` | テーブル名の文字列リテラル型（例: &quot;users&quot;）。 |

#### 定義場所 \{#defined-in\}

[values/value.ts:52](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L52)

***

### Value \{#value\}

Ƭ **Value**: `null` | `bigint` | `number` | `boolean` | `string` | `ArrayBuffer` | [`Value`](values.md#value)[] | &#123; `[key: string]`: `undefined` | [`Value`](values.md#value);  &#125;

Convex がサポートする値です。

値は次のように利用できます:

* ドキュメント内に保存する。
* クエリ関数やミューテーション関数の引数や戻り値の型として使用する。

サポートされている型の一覧は
[Types](https://docs.convex.dev/using/types)
を参照してください。

#### 定義場所 \{#defined-in\}

[values/value.ts:66](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L66)

***

### NumericValue \{#numericvalue\}

Ƭ **NumericValue**: `bigint` | `number`

数値を表すために使用できる[値](values.md#value)型。

#### 定義元 \{#defined-in\}

[values/value.ts:81](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L81)

## 変数 \{#variables\}

### v \{#v\}

• `Const` **v**: `Object`

バリデーターを生成するビルダーです。

このビルダーを使うと、Convex の値に対するバリデーターを構築できます。

バリデーターは[スキーマ定義](https://docs.convex.dev/database/schemas)や、
Convex 関数の入力バリデーターとして利用できます。

#### 型宣言 \{#type-declaration\}

| 名前 | 型 |
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

#### 定義場所 \{#defined-in\}

[values/validator.ts:80](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L80)

## 関数 \{#functions\}

### compareValues \{#comparevalues\}

▸ **compareValues**(`k1`, `k2`): `number`

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `k1` | `undefined` | [`値`](values.md#value) |
| `k2` | `undefined` | [`値`](values.md#value) |

#### 戻り値 \{#returns\}

`number`

#### 定義場所 \{#defined-in\}

[values/compare.ts:4](https://github.com/get-convex/convex-js/blob/main/src/values/compare.ts#L4)

***

### asObjectValidator \{#asobjectvalidator\}

▸ **asObjectValidator**&lt;`V`&gt;(`obj`): `V` extends [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; ? `V` : `V` extends [`PropertyValidators`](values.md#propertyvalidators) ? [`Validator`](values.md#validator)&lt;[`ObjectType`](values.md#objecttype)&lt;`V`&gt;&gt; : `never`

プロパティとしてバリデータを持つオブジェクトを、バリデータに変換します。
バリデータが渡された場合は、そのまま返します。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `V` | extends [`PropertyValidators`](values.md#propertyvalidators) | [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; |

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `obj` | `V` |

#### 戻り値 \{#returns\}

`V` extends [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; ? `V` : `V` extends [`PropertyValidators`](values.md#propertyvalidators) ? [`Validator`](values.md#validator)&lt;[`ObjectType`](values.md#objecttype)&lt;`V`&gt;&gt; : `never`

#### 定義元 \{#defined-in\}

[values/validator.ts:39](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L39)

***

### jsonToConvex \{#jsontoconvex\}

▸ **jsonToConvex**(`value`): [`Value`](values.md#value)

JSON 表現から Convex の値をパースします。

この関数は、シリアル化された Int64 を `BigInt` に、Bytes を `ArrayBuffer` などにデシリアライズします。

Convex の値の詳細については、[Types](https://docs.convex.dev/using/types) を参照してください。

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `value` | [`JSONValue`](values.md#jsonvalue) | 以前に [convexToJson](values.md#convextojson) を使用して作成した Convex の値の JSON 表現。 |

#### 戻り値 \{#returns\}

[`値`](values.md#value)

Convex の値を表す JavaScript 上の表現です。

#### 定義場所 \{#defined-in\}

[values/value.ts:187](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L187)

***

### convexToJson \{#convextojson\}

▸ **convexToJson**(`value`): [`JSONValue`](values.md#jsonvalue)

Convex の値を JSON 表現に変換します。

元の値を復元するには [jsonToConvex](values.md#jsontoconvex) を使用します。

Convex の値について詳しくは、[Types](https://docs.convex.dev/using/types) を参照してください。

#### パラメータ \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `value` | [`Value`](values.md#value) | JSON に変換する Convex の値。 |

#### 戻り値 \{#returns\}

[`JSONValue`](values.md#jsonvalue)

`value` の JSON 表現。

#### 定義元 \{#defined-in\}

[values/value.ts:429](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L429)