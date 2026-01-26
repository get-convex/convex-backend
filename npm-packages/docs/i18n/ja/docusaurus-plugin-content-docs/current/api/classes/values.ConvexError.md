---
id: "values.ConvexError"
title: "クラス: ConvexError<TData>"
custom_edit_url: null
---

[values](../modules/values.md).ConvexError

## 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TData` | extends [`Value`](../modules/values.md#value) |

## 継承関係 \{#hierarchy\}

* `Error`

  ↳ **`ConvexError`**

## コンストラクタ \{#constructors\}

### コンストラクタ \{#constructor\}

• **new ConvexError**&lt;`TData`&gt;(`data`)

#### 型パラメーター \{#type-parameters\}

| 名前 | 型 |
| :------ | :------ |
| `TData` | extends [`Value`](../modules/values.md#value) |

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `data` | `TData` |

#### オーバーライド \{#overrides\}

Error.constructor

#### 定義元 \{#defined-in\}

[values/errors.ts:10](https://github.com/get-convex/convex-js/blob/main/src/values/errors.ts#L10)

## プロパティ \{#properties\}

### stackTraceLimit \{#stacktracelimit\}

▪ `Static` **stackTraceLimit**: `number`

`Error.stackTraceLimit` プロパティは、スタックトレースに含められるスタックフレームの数を指定します
（`new Error().stack` や `Error.captureStackTrace(obj)` によって生成された場合を含みます）。

デフォルト値は `10` ですが、有効な任意の JavaScript 数値に設定できます。値を変更すると、
その変更後に取得されるスタックトレースに影響します。

数値以外の値、または負の数値が設定された場合、スタックトレースにはフレームが一切含まれません。

#### 継承元 \{#inherited-from\}

Error.stackTraceLimit

#### 定義元 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/@types+node@18.19.130/node&#95;modules/@types/node/globals.d.ts:68

***

### cause \{#cause\}

• `Optional` **cause**: `unknown`

#### 継承元 \{#inherited-from\}

Error.cause

#### 定義場所 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es2022.error.d.ts:24

***

### message \{#message\}

• **message**: `string`

#### 継承元 \{#inherited-from\}

Error.message

#### 定義場所 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es5.d.ts:1055

***

### stack \{#stack\}

• `Optional` **stack**: `string`

#### 継承元 \{#inherited-from\}

Error.stack

#### 定義場所 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es5.d.ts:1056

***

### name \{#name\}

• **name**: `string` = `"ConvexError"`

#### オーバーライド \{#overrides\}

Error.name

#### 定義元 \{#defined-in\}

[values/errors.ts:6](https://github.com/get-convex/convex-js/blob/main/src/values/errors.ts#L6)

***

### data \{#data\}

• **data**: `TData`

#### 定義場所 \{#defined-in\}

[values/errors.ts:7](https://github.com/get-convex/convex-js/blob/main/src/values/errors.ts#L7)

***

### [IDENTIFYING_FIELD]

• **[IDENTIFYING&#95;FIELD]**: `boolean` = `true`

#### 定義元 \{#defined-in\}

[values/errors.ts:8](https://github.com/get-convex/convex-js/blob/main/src/values/errors.ts#L8)

## メソッド \{#methods\}

### captureStackTrace \{#capturestacktrace\}

▸ `Static` **captureStackTrace**(`targetObject`, `constructorOpt?`): `void`

`targetObject` に `.stack` プロパティを作成します。このプロパティにアクセスすると、
`Error.captureStackTrace()` が呼び出されたコード内の位置を表す文字列が返されます。

```js
const myObject = {};
Error.captureStackTrace(myObject);
myObject.stack;  // `new Error().stack` と同様
```

トレースの最初の行には `${myObject.name}: ${myObject.message}` が前置されます。

オプションの `constructorOpt` 引数には関数を渡せます。指定された場合、
`constructorOpt` を含め、それより上のすべてのフレームは、生成されるスタック
トレースから省略されます。

`constructorOpt` 引数は、エラー生成の実装の詳細をユーザーから隠すのに便利です。
たとえば、次のように使います:

```js
function a() {
  b();
}

function b() {
  c();
}

function c() {
  // Create an error without stack trace to avoid calculating the stack trace twice.
  const { stackTraceLimit } = Error;
  Error.stackTraceLimit = 0;
  const error = new Error();
  Error.stackTraceLimit = stackTraceLimit;

  // Capture the stack trace above function b
  Error.captureStackTrace(error, b); // 関数 c も b もスタックトレースには含まれません
  throw error;
}

a();
```

#### パラメーター \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `targetObject` | `object` |
| `constructorOpt?` | `Function` |

#### 戻り値 \{#returns\}

`void`

#### 継承元 \{#inherited-from\}

Error.captureStackTrace

#### 定義場所 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/@types+node@18.19.130/node&#95;modules/@types/node/globals.d.ts:52

***

### prepareStackTrace \{#preparestacktrace\}

▸ `Static` **prepareStackTrace**(`err`, `stackTraces`): `any`

**`参照`**

https://v8.dev/docs/stack-trace-api#customizing-stack-traces

#### パラメータ \{#parameters\}

| 名前 | 型 |
| :------ | :------ |
| `err` | `Error` |
| `stackTraces` | `CallSite`[] |

#### 戻り値 \{#returns\}

`any`

#### 継承元 \{#inherited-from\}

Error.prepareStackTrace

#### 定義場所 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/@types+node@18.19.130/node&#95;modules/@types/node/globals.d.ts:56