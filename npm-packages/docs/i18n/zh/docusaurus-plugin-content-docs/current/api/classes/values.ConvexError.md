---
id: "values.ConvexError"
title: "类：ConvexError<TData>"
custom_edit_url: null
---

[values](../modules/values.md).ConvexError

## 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TData` | extends [`Value`](../modules/values.md#value) |

## 继承层次结构 \{#hierarchy\}

* `Error`

  ↳ **`ConvexError`**

## 构造函数 \{#constructors\}

### 构造函数 \{#constructor\}

• **new ConvexError**&lt;`TData`&gt;(`data`)

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TData` | extends [`Value`](../modules/values.md#value) |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `data` | `TData` |

#### 重写 \{#overrides\}

Error.constructor

#### 定义于 \{#defined-in\}

[values/errors.ts:10](https://github.com/get-convex/convex-js/blob/main/src/values/errors.ts#L10)

## 属性 \{#properties\}

### stackTraceLimit \{#stacktracelimit\}

▪ `Static` **stackTraceLimit**: `number`

`Error.stackTraceLimit` 属性指定堆栈跟踪中收集的调用栈帧数量（无论是由 `new Error().stack` 还是 `Error.captureStackTrace(obj)` 生成）。

默认值为 `10`，但可以设置为任意有效的 JavaScript 数值。对该值的更改会影响在更改&#95;之后&#95;捕获的任何堆栈跟踪。

如果设置为非数值，或者设置为负数，堆栈跟踪将不会捕获任何帧。

#### 继承自 \{#inherited-from\}

Error.stackTraceLimit

#### 定义于 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/@types+node@18.19.130/node&#95;modules/@types/node/globals.d.ts:68

***

### cause \{#cause\}

• `可选` **cause**：`unknown`

#### 继承自 \{#inherited-from\}

Error.cause

#### 定义于 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es2022.error.d.ts:24

***

### message \{#message\}

• **message**: `string`

#### 继承自 \{#inherited-from\}

Error.message

#### 定义于 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es5.d.ts:1055

***

### stack \{#stack\}

• `可选` **stack**: `string`

#### 继承自 \{#inherited-from\}

Error.stack

#### 定义在 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/typescript@5.0.4/node&#95;modules/typescript/lib/lib.es5.d.ts:1056

***

### name \{#name\}

• **name**: `string` = `"ConvexError"`

#### 重写 \{#overrides\}

Error.name

#### 定义于 \{#defined-in\}

[values/errors.ts:6](https://github.com/get-convex/convex-js/blob/main/src/values/errors.ts#L6)

***

### data \{#data\}

• **data**: `TData`

#### 定义于 \{#defined-in\}

[values/errors.ts:7](https://github.com/get-convex/convex-js/blob/main/src/values/errors.ts#L7)

***

### [IDENTIFYING_FIELD]

• **[IDENTIFYING&#95;FIELD]**: `boolean` = `true`

#### 定义于 \{#defined-in\}

[values/errors.ts:8](https://github.com/get-convex/convex-js/blob/main/src/values/errors.ts#L8)

## 方法 \{#methods\}

### captureStackTrace \{#capturestacktrace\}

▸ `Static` **captureStackTrace**(`targetObject`, `constructorOpt?`): `void`

在 `targetObject` 上创建一个 `.stack` 属性，在访问该属性时，
会返回一个字符串，表示调用 `Error.captureStackTrace()` 时的代码位置。

```js
const myObject = {};
Error.captureStackTrace(myObject);
myObject.stack;  // 类似于 `new Error().stack`
```

堆栈跟踪的第一行前面会加上
`${myObject.name}: ${myObject.message}` 作为前缀。

可选的 `constructorOpt` 参数接受一个函数。如果提供了该参数，位于
`constructorOpt` 之上的所有调用栈帧（包括 `constructorOpt` 本身）都不会出现在生成的
堆栈跟踪中。

`constructorOpt` 参数对于向用户隐藏错误生成的实现细节非常有用。例如：

```js
function a() {
  b();
}

function b() {
  c();
}

function c() {
  // 创建一个不带堆栈跟踪的错误,避免重复计算堆栈跟踪。
  const { stackTraceLimit } = Error;
  Error.stackTraceLimit = 0;
  const error = new Error();
  Error.stackTraceLimit = stackTraceLimit;

  // 捕获函数 b 上方的堆栈跟踪
  Error.captureStackTrace(error, b); // 函数 c 和 b 都不会包含在堆栈跟踪中
  throw error;
}

a();
```

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `targetObject` | `object` |
| `constructorOpt?` | `Function` |

#### 返回值 \{#returns\}

`void`

#### 继承自 \{#inherited-from\}

Error.captureStackTrace

#### 定义于 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/@types+node@18.19.130/node&#95;modules/@types/node/globals.d.ts:52

***

### prepareStackTrace \{#preparestacktrace\}

▸ `Static` **prepareStackTrace**(`err`, `stackTraces`): `any`

**`参见`**

https://v8.dev/docs/stack-trace-api#customizing-stack-traces

#### 参数 \{#parameters\}

| 参数名 | 类型 |
| :------ | :------ |
| `err` | `Error` |
| `stackTraces` | `CallSite`[] |

#### 返回值 \{#returns\}

`any`

#### 继承自 \{#inherited-from\}

Error.prepareStackTrace

#### 定义于 \{#defined-in\}

../../common/temp/node&#95;modules/.pnpm/@types+node@18.19.130/node&#95;modules/@types/node/globals.d.ts:56