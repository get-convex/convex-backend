---
id: "values"
title: "Módulo: values"
custom_edit_url: null
---

Utilidades para trabajar con valores almacenados en Convex.

Puedes consultar el conjunto completo de tipos admitidos en
[Tipos](https://docs.convex.dev/using/types).

## Espacios de nombres \{#namespaces\}

* [Base64](../namespaces/values.Base64.md)

## Clases \{#classes\}

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

## Alias de tipos \{#type-aliases\}

### GenericValidator \{#genericvalidator\}

Ƭ **GenericValidator**: [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt;

El tipo base que todos los validadores deben extender.

#### Definido en \{#defined-in\}

[values/validator.ts:27](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L27)

***

### AsObjectValidator \{#asobjectvalidator\}

Ƭ **AsObjectValidator**&lt;`V`&gt;: `V` extends [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; ? `V` : `V` extends [`PropertyValidators`](values.md#propertyvalidators) ? [`Validator`](values.md#validator)&lt;[`ObjectType`](values.md#objecttype)&lt;`V`&gt;&gt; : `never`

Convierte un objeto con validadores como propiedades en un validador.
Si se pasa un validador, se devuelve ese mismo.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `V` | extends [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |

#### Definido en \{#defined-in\}

[values/validator.ts:61](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L61)

***

### PropertyValidators \{#propertyvalidators\}

Ƭ **PropertyValidators**: `Record`&lt;`string`, [`Validator`](values.md#validator)&lt;`any`, [`OptionalProperty`](values.md#optionalproperty), `any`&gt;&gt;

Validadores para cada propiedad de un objeto.

Esto se representa como un objeto que asigna el nombre de la propiedad a su
[Validator](values.md#validator).

#### Definido en \{#defined-in\}

[values/validator.ts:242](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L242)

***

### ObjectType \{#objecttype\}

Ƭ **ObjectType**&lt;`Fields`&gt;: [`Expand`](server.md#expand)&lt;&#123; [Property in OptionalKeys&lt;Fields&gt;]?: Exclude&lt;Infer&lt;Fields[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in RequiredKeys&lt;Fields&gt;]: Infer&lt;Fields[Property]&gt; &#125;&gt;

Calcula el tipo de un objeto a partir de [PropertyValidators](values.md#propertyvalidators).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Fields` | extiende [`PropertyValidators`](values.md#propertyvalidators) |

#### Definido en \{#defined-in\}

[values/validator.ts:252](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L252)

***

### Infer \{#infer\}

Ƭ **Infer**&lt;`T`&gt;: `T`[`"type"`]

Extrae un tipo de TypeScript a partir de un validador.

Ejemplo de uso:

```ts
const objectSchema = v.object({
  property: v.string(),
});
type MyObject = Infer<typeof objectSchema>; // { property: string }
```

**`Type Param`**

El tipo del [Validator](values.md#validator) construido con [v](values.md#v).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extends [`Validator`](values.md#validator)&lt;`any`, [`OptionalProperty`](values.md#optionalproperty), `any`&gt; |

#### Definido en \{#defined-in\}

[values/validator.ts:294](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L294)

***

### VOptional \{#voptional\}

Ƭ **VOptional**&lt;`T`&gt;: `T` extends [`VId`](../classes/values.VId.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VId`](../classes/values.VId.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VString`](../classes/values.VString.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VString`](../classes/values.VString.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VFloat64`](../classes/values.VFloat64.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VFloat64`](../classes/values.VFloat64.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VInt64`](../classes/values.VInt64.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VInt64`](../classes/values.VInt64.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VBoolean`](../classes/values.VBoolean.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VBoolean`](../classes/values.VBoolean.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VNull`](../classes/values.VNull.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VNull`](../classes/values.VNull.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VAny`](../classes/values.VAny.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VAny`](../classes/values.VAny.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VLiteral`](../classes/values.VLiteral.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VLiteral`](../classes/values.VLiteral.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VBytes`](../classes/values.VBytes.md)&lt;infer Type, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VBytes`](../classes/values.VBytes.md)&lt;`Type` | `undefined`, `"optional"`&gt; : `T` extends [`VObject`](../classes/values.VObject.md)&lt;infer Type, infer Fields, [`OptionalProperty`](values.md#optionalproperty), infer FieldPaths&gt; ? [`VObject`](../classes/values.VObject.md)&lt;`Type` | `undefined`, `Fields`, `"optional"`, `FieldPaths`&gt; : `T` extends [`VArray`](../classes/values.VArray.md)&lt;infer Type, infer Element, [`OptionalProperty`](values.md#optionalproperty)&gt; ? [`VArray`](../classes/values.VArray.md)&lt;`Type` | `undefined`, `Element`, `"optional"`&gt; : `T` extends [`VRecord`](../classes/values.VRecord.md)&lt;infer Type, infer Key, infer Value, [`OptionalProperty`](values.md#optionalproperty), infer FieldPaths&gt; ? [`VRecord`](../classes/values.VRecord.md)&lt;`Type` | `undefined`, `Key`, `Value`, `"optional"`, `FieldPaths`&gt; : `T` extends [`VUnion`](../classes/values.VUnion.md)&lt;infer Type, infer Members, [`OptionalProperty`](values.md#optionalproperty), infer FieldPaths&gt; ? [`VUnion`](../classes/values.VUnion.md)&lt;`Type` | `undefined`, `Members`, `"optional"`, `FieldPaths`&gt; : `never`

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extends [`Validator`](values.md#validator)&lt;`any`, [`OptionalProperty`](values.md#optionalproperty), `any`&gt; |

#### Definido en \{#defined-in\}

[values/validators.ts:648](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L648)

***

### OptionalProperty \{#optionalproperty\}

Ƭ **OptionalProperty**: `"optional"` | `"required"`

Tipo que representa si una propiedad en un objeto es opcional o requerida.

#### Definido en \{#defined-in\}

[values/validators.ts:681](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L681)

***

### Validator \{#validator\}

Ƭ **Validator**&lt;`Type`, `IsOptional`, `FieldPaths`&gt;: [`VId`](../classes/values.VId.md)&lt;`Type`, `IsOptional`&gt; | [`VString`](../classes/values.VString.md)&lt;`Type`, `IsOptional`&gt; | [`VFloat64`](../classes/values.VFloat64.md)&lt;`Type`, `IsOptional`&gt; | [`VInt64`](../classes/values.VInt64.md)&lt;`Type`, `IsOptional`&gt; | [`VBoolean`](../classes/values.VBoolean.md)&lt;`Type`, `IsOptional`&gt; | [`VNull`](../classes/values.VNull.md)&lt;`Type`, `IsOptional`&gt; | [`VAny`](../classes/values.VAny.md)&lt;`Type`, `IsOptional`&gt; | [`VLiteral`](../classes/values.VLiteral.md)&lt;`Type`, `IsOptional`&gt; | [`VBytes`](../classes/values.VBytes.md)&lt;`Type`, `IsOptional`&gt; | [`VObject`](../classes/values.VObject.md)&lt;`Type`, `Record`&lt;`string`, [`Validator`](values.md#validator)&lt;`any`, [`OptionalProperty`](values.md#optionalproperty), `any`&gt;&gt;, `IsOptional`, `FieldPaths`&gt; | [`VArray`](../classes/values.VArray.md)&lt;`Type`, [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt;, `IsOptional`&gt; | [`VRecord`](../classes/values.VRecord.md)&lt;`Type`, [`Validator`](values.md#validator)&lt;`string`, `"required"`, `any`&gt;, [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt;, `IsOptional`, `FieldPaths`&gt; | [`VUnion`](../classes/values.VUnion.md)&lt;`Type`, [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt;[], `IsOptional`, `FieldPaths`&gt;

Un validador para un Valor de Convex.

Debe construirse usando el generador de validadores, [v](values.md#v).

Un validador encapsula:

* El tipo de TypeScript de este valor.
* Si este campo debe ser opcional cuando se incluye en un objeto.
* El tipo de TypeScript para el conjunto de rutas de campos de índice que se pueden usar para
  construir índices para este valor.
* Una representación JSON del validador.

Tipos específicos de validadores contienen información adicional: por ejemplo,
un `ArrayValidator` contiene una propiedad `element` con el validador
usado para validar cada elemento de la lista. Usa la propiedad compartida `kind`
para identificar el tipo de validador.

Es posible que se agreguen más validadores en versiones futuras, por lo que se debe esperar
que una instrucción `switch` exhaustiva sobre el `kind` del validador deje de funcionar
en futuras versiones de Convex.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | `Type` |
| `IsOptional` | extends [`OptionalProperty`](values.md#optionalproperty) = `"required"` |
| `FieldPaths` | extends `string` = `never` |

#### Definido en \{#defined-in\}

[values/validators.ts:706](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L706)

***

### ObjectFieldType \{#objectfieldtype\}

Ƭ **ObjectFieldType**: `Object`

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo |
| :------ | :------ |
| `fieldType` | [`ValidatorJSON`](values.md#validatorjson) |
| `optional` | `boolean` |

#### Definido en \{#defined-in\}

[values/validators.ts:747](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L747)

***

### ValidatorJSON \{#validatorjson\}

Ƭ **ValidatorJSON**: &#123; `type`: `"null"`  &#125; | &#123; `type`: `"number"`  &#125; | &#123; `type`: `"bigint"`  &#125; | &#123; `type`: `"boolean"`  &#125; | &#123; `type`: `"string"`  &#125; | &#123; `type`: `"bytes"`  &#125; | &#123; `type`: `"any"`  &#125; | &#123; `type`: `"literal"` ; `value`: [`JSONValue`](values.md#jsonvalue)  &#125; | &#123; `type`: `"id"` ; `tableName`: `string`  &#125; | &#123; `type`: `"array"` ; `value`: [`ValidatorJSON`](values.md#validatorjson)  &#125; | &#123; `type`: `"record"` ; `keys`: [`RecordKeyValidatorJSON`](values.md#recordkeyvalidatorjson) ; `values`: [`RecordValueValidatorJSON`](values.md#recordvaluevalidatorjson)  &#125; | &#123; `type`: `"object"` ; `value`: `Record`&lt;`string`, [`ObjectFieldType`](values.md#objectfieldtype)&gt;  &#125; | &#123; `type`: `"union"` ; `value`: [`ValidatorJSON`](values.md#validatorjson)[]  &#125;

#### Definido en \{#defined-in\}

[values/validators.ts:749](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L749)

***

### RecordKeyValidatorJSON \{#recordkeyvalidatorjson\}

Ƭ **RecordKeyValidatorJSON**: &#123; `type`: `"string"`  &#125; | &#123; `type`: `"id"` ; `tableName`: `string`  &#125; | &#123; `type`: `"union"` ; `value`: [`RecordKeyValidatorJSON`](values.md#recordkeyvalidatorjson)[]  &#125;

#### Definido en \{#defined-in\}

[values/validators.ts:768](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L768)

***

### RecordValueValidatorJSON \{#recordvaluevalidatorjson\}

Ƭ **RecordValueValidatorJSON**: [`ObjectFieldType`](values.md#objectfieldtype) &amp; &#123; `optional`: `false`  &#125;

#### Definido en \{#defined-in\}

[values/validators.ts:773](https://github.com/get-convex/convex-js/blob/main/src/values/validators.ts#L773)

***

### JSONValue \{#jsonvalue\}

Ƭ **JSONValue**: `null` | `boolean` | `number` | `string` | [`JSONValue`](values.md#jsonvalue)[] | &#123; `[key: string]`: [`JSONValue`](values.md#jsonvalue);  &#125;

El tipo de valores de JavaScript que se pueden serializar a JSON.

#### Definido en \{#defined-in\}

[values/value.ts:24](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L24)

***

### GenericId \{#genericid\}

Ƭ **GenericId**&lt;`TableName`&gt;: `string` &amp; &#123; `__tableName`: `TableName`  &#125;

Un identificador de un documento en Convex.

Los documentos de Convex se identifican de forma única mediante su `Id`, accesible
en el campo `_id`. Para obtener más información, consulta [Document IDs](https://docs.convex.dev/database/document-ids).

Los documentos se pueden cargar usando `db.get(tableName, id)` en funciones de consulta y mutación.

Los IDs son cadenas codificadas en base 32 que son seguras para usar en URLs.

Los IDs son solo cadenas en tiempo de ejecución, pero este tipo se puede usar para distinguirlos de otras
cadenas en tiempo de compilación.

Si estás usando generación de código, usa el tipo `Id` generado para tu modelo de datos en
`convex/_generated/dataModel.d.ts`.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `TableName` | extends `string` | Un tipo de cadena literal que representa el nombre de la tabla (como &quot;users&quot;). |

#### Definido en \{#defined-in\}

[values/value.ts:52](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L52)

***

### Valor \{#value\}

Ƭ **Value**: `null` | `bigint` | `number` | `boolean` | `string` | `ArrayBuffer` | [`Value`](values.md#value)[] | &#123; `[key: string]`: `undefined` | [`Value`](values.md#value);  &#125;

Un valor admitido por Convex.

Los valores pueden:

* almacenarse dentro de documentos.
* usarse como argumentos y tipos de retorno de funciones de consulta y de mutación.

Puedes ver el conjunto completo de tipos admitidos en
[Tipos](https://docs.convex.dev/using/types).

#### Definido en \{#defined-in\}

[values/value.ts:66](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L66)

***

### NumericValue \{#numericvalue\}

Ƭ **NumericValue**: `bigint` | `number`

Tipos de [Valor](values.md#value) que pueden usarse para representar números.

#### Definido en \{#defined-in\}

[values/value.ts:81](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L81)

## Variables \{#variables\}

### v \{#v\}

• `Const` **v**: `Object`

El constructor de validadores.

Este constructor te permite crear validadores para valores de Convex.

Los validadores se pueden usar en [definiciones de esquema](https://docs.convex.dev/database/schemas)
y como validadores de entrada para funciones de Convex.

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo |
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

#### Definido en \{#defined-in\}

[values/validator.ts:80](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L80)

## Funciones \{#functions\}

### compareValues \{#comparevalues\}

▸ **compareValues**(`k1`, `k2`): `number`

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `k1` | `undefined` | [`Valor`](values.md#value) |
| `k2` | `undefined` | [`Valor`](values.md#value) |

#### Devuelve \{#returns\}

`number`

#### Definido en \{#defined-in\}

[values/compare.ts:4](https://github.com/get-convex/convex-js/blob/main/src/values/compare.ts#L4)

***

### asObjectValidator \{#asobjectvalidator\}

▸ **asObjectValidator**&lt;`V`&gt;(`obj`): `V` extends [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; ? `V` : `V` extends [`PropertyValidators`](values.md#propertyvalidators) ? [`Validator`](values.md#validator)&lt;[`ObjectType`](values.md#objecttype)&lt;`V`&gt;&gt; : `never`

Convierte un objeto con validadores como propiedades en un validador.
Si se pasa un validador, se devuelve tal cual.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `V` | extends [`PropertyValidators`](values.md#propertyvalidators) | [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `obj` | `V` |

#### Devuelve \{#returns\}

`V` extends [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; ? `V` : `V` extends [`PropertyValidators`](values.md#propertyvalidators) ? [`Validator`](values.md#validator)&lt;[`ObjectType`](values.md#objecttype)&lt;`V`&gt;&gt; : `never`

#### Definido en \{#defined-in\}

[values/validator.ts:39](https://github.com/get-convex/convex-js/blob/main/src/values/validator.ts#L39)

***

### jsonToConvex \{#jsontoconvex\}

▸ **jsonToConvex**(`value`): [`Value`](values.md#value)

Analiza un Valor de Convex a partir de su representación en JSON.

Esta función deserializa Int64 serializados en `BigInt`, Bytes en `ArrayBuffer`, etc.

Para obtener más información sobre los valores de Convex, consulta [Tipos](https://docs.convex.dev/using/types).

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `value` | [`JSONValue`](values.md#jsonvalue) | La representación en JSON de un Valor de Convex creado previamente con [convexToJson](values.md#convextojson). |

#### Returns \{#returns\}

[`Valor`](values.md#value)

La representación en JavaScript del Valor de Convex.

#### Definido en \{#defined-in\}

[values/value.ts:187](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L187)

***

### convexToJson \{#convextojson\}

▸ **convexToJson**(`value`): [`JSONValue`](values.md#jsonvalue)

Convierte un valor de Convex a su representación JSON.

Utiliza [jsonToConvex](values.md#jsontoconvex) para recrear el valor original.

Para obtener más información sobre los valores de Convex, consulta [Tipos](https://docs.convex.dev/using/types).

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `value` | [`Valor`](values.md#value) | Un Valor de Convex que se convertirá a JSON. |

#### Devuelve \{#returns\}

[`JSONValue`](values.md#jsonvalue)

La representación JSON de `value`.

#### Definido en \{#defined-in\}

[values/value.ts:429](https://github.com/get-convex/convex-js/blob/main/src/values/value.ts#L429)