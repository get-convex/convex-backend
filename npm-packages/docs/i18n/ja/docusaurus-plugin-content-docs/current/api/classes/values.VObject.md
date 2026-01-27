---
id: "values.VObject"
title: "クラス: VObject<Type, Fields, IsOptional, FieldPaths>"
custom_edit_url: null
---

[values](../modules/values.md).VObject

`v.object()` バリデータの型。

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `Type` |
| `Fields` | extends `Record`&lt;`string`, [`GenericValidator`](../modules/values.md#genericvalidator)&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = &#123; [Property in keyof Fields]: JoinFieldPaths&lt;Property &amp; string, Fields[Property][&quot;fieldPaths&quot;]&gt; | Property &#125;[keyof `Fields`] &amp; `string` |

## 継承階層 \{#hierarchy\}

* `BaseValidator`&lt;`Type`, `IsOptional`, `FieldPaths`&gt;

  ↳ **`VObject`**

## コンストラクタ \{#constructors\}

### constructor \{#constructor\}

• **new VObject**&lt;`Type`, `Fields`, `IsOptional`, `FieldPaths`&gt;(`«destructured»`)

通常は `v.object({ ... })` を使用します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `Type` | `Type` |
| `Fields` | extends `Record`&lt;`string`, [`GenericValidator`](../modules/values.md#genericvalidator)&gt; |
| `IsOptional` | extends [`OptionalProperty`](../modules/values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Fields[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof `Fields`] &amp; `string` |

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `«destructured»` | `Object` |
| › `isOptional` | `IsOptional` |
| › `fields` | `Fields` |

#### オーバーライド \{#overrides\}

BaseValidator&lt;Type, IsOptional, FieldPaths&gt;.constructor

#### 定義元 \{#defined-in\}

[values/validators.ts:304](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L304)

## プロパティ \{#properties\}

### type \{#type\}

• `Readonly` **type**: `Type`

TypeScript の場合のみ有効で、このバリデータによって検証される JS の値の TS 型です。

#### 継承元 \{#inherited-from\}

BaseValidator.type

#### 定義元 \{#defined-in\}

[values/validators.ts:37](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L37)

***

### fieldPaths \{#fieldpaths\}

• `Readonly` **fieldPaths**: `FieldPaths`

TypeScript のみで使用されます。これが Object 用バリデータの場合、
そのプロパティ名の TS 型になります。

#### 継承元 \{#inherited-from\}

BaseValidator.fieldPaths

#### 定義元 \{#defined-in\}

[values/validators.ts:42](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L42)

***

### isOptional \{#isoptional\}

• `Readonly` **isOptional**: `IsOptional`

これが Object のオプションのプロパティ値バリデータかどうかを示します。

#### 継承元 \{#inherited-from\}

BaseValidator.isOptional

#### 定義場所 \{#defined-in\}

[values/validators.ts:47](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L47)

***

### isConvexValidator \{#isconvexvalidator\}

• `Readonly` **isConvexValidator**: `true`

常に `"true"` です。

#### 継承元 \{#inherited-from\}

BaseValidator.isConvexValidator

#### 定義場所 \{#defined-in\}

[values/validators.ts:52](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L52)

***

### fields \{#fields\}

• `Readonly` **fields**: `Fields`

各プロパティ用のバリデータを持つオブジェクト。

#### 定義場所 \{#defined-in\}

[values/validators.ts:294](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L294)

***

### kind \{#kind\}

• `Readonly` **kind**: `"object"`

バリデータの種類で、`"object"` を表します。

#### 定義場所 \{#defined-in\}

[values/validators.ts:299](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L299)

## メソッド \{#methods\}

### omit \{#omit\}

▸ **omit**&lt;`K`&gt;(`...fields`): [`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Type`, `K`&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Fields`, `K`&gt;&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Omit&lt;Fields, K&gt;&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Fields`, `K`&gt;&gt;] &amp; `string`&gt;

指定したフィールドを除外した新しい VObject を作成します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `K` | extends `string` |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `...fields` | `K`[] | この VObject から除外するフィールド名。 |

#### 戻り値 \{#returns\}

[`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Type`, `K`&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Fields`, `K`&gt;&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Omit&lt;Fields, K&gt;&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Omit`&lt;`Fields`, `K`&gt;&gt;] &amp; `string`&gt;

#### 定義元 \{#defined-in\}

[values/validators.ts:349](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L349)

***

### pick \{#pick\}

▸ **pick**&lt;`K`&gt;(`...fields`): [`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Type`, `Extract`&lt;keyof `Type`, `K`&gt;&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Fields`, `K`&gt;&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Pick&lt;Fields, K&gt;&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Fields`, `K`&gt;&gt;] &amp; `string`&gt;

指定したフィールドのみを含む新しい VObject を作成します。

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `K` | `string` を拡張 |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `...fields` | `K`[] | この VObject から選択するフィールド名。 |

#### 戻り値 \{#returns\}

[`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Type`, `Extract`&lt;keyof `Type`, `K`&gt;&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Fields`, `K`&gt;&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Pick&lt;Fields, K&gt;&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Pick`&lt;`Fields`, `K`&gt;&gt;] &amp; `string`&gt;

#### 定義場所 \{#defined-in\}

[values/validators.ts:366](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L366)

***

### partial \{#partial\}

▸ **partial**(): [`VObject`](values.VObject.md)&lt;&#123; [K in string | number | symbol]?: Type[K] &#125;, &#123; [K in string | number | symbol]: VOptional&lt;Fields[K]&gt; &#125;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;&#123; [K in string | number | symbol]: VOptional&lt;Fields[K]&gt; &#125;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof `Fields`] &amp; `string`&gt;

すべてのフィールドがオプショナルな新しい `VObject` を作成します。

#### 戻り値 \{#returns\}

[`VObject`](values.VObject.md)&lt;&#123; [K in string | number | symbol]?: Type[K] &#125;, &#123; [K in string | number | symbol]: VOptional&lt;Fields[K]&gt; &#125;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;&#123; [K in string | number | symbol]: VOptional&lt;Fields[K]&gt; &#125;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof `Fields`] &amp; `string`&gt;

#### 定義場所 \{#defined-in\}

[values/validators.ts:386](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L386)

***

### extend \{#extend\}

▸ **extend**&lt;`NewFields`&gt;(`fields`): [`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Type` &amp; [`ObjectType`](../modules/values.md#objecttype)&lt;`NewFields`&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Fields` &amp; `NewFields`&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Fields &amp; NewFields&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Fields` &amp; `NewFields`&gt;] &amp; `string`&gt;

追加フィールドをマージして新しい `VObject` を作成します。

#### 型パラメータ \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `NewFields` | extends `Record`&lt;`string`, [`GenericValidator`](../modules/values.md#genericvalidator)&gt; |

#### パラメーター \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `fields` | `NewFields` | この VObject にマージするための追加のバリデーターを含むオブジェクト。 |

#### 戻り値 \{#returns\}

[`VObject`](values.VObject.md)&lt;[`Expand`](../modules/server.md#expand)&lt;`Type` &amp; [`ObjectType`](../modules/values.md#objecttype)&lt;`NewFields`&gt;&gt;, [`Expand`](../modules/server.md#expand)&lt;`Fields` &amp; `NewFields`&gt;, `IsOptional`, &#123; [Property in string | number | symbol]: Property | `$&#123;Property &amp; string&#125;.$&#123;Expand&lt;Fields &amp; NewFields&gt;[Property][&quot;fieldPaths&quot;]&#125;` &#125;[keyof [`Expand`](../modules/server.md#expand)&lt;`Fields` &amp; `NewFields`&gt;] &amp; `string`&gt;

#### 定義元 \{#defined-in\}

[values/validators.ts:407](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L407)