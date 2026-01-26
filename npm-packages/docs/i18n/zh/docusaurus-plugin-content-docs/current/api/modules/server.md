---
id: "server"
title: "模块：server"
custom_edit_url: null
---

用于实现服务端 Convex 查询和变更函数的工具。

## 用法 \{#usage\}

### 代码生成 \{#code-generation\}

此模块通常与生成的服务端代码一起使用。

要生成服务端代码，请在你的 Convex 项目中运行 `npx convex dev`。
这将创建一个 `convex/_generated/server.js` 文件，其中包含以下
根据你的模式添加了类型标注的函数：

* [query](https://docs.convex.dev/generated-api/server#query)
* [mutation](https://docs.convex.dev/generated-api/server#mutation)

如果你不使用 TypeScript 和代码生成，可以改用这些未加类型标注的
函数：

* [queryGeneric](server.md#querygeneric)
* [mutationGeneric](server.md#mutationgeneric)

### 示例 \{#example\}

Convex 函数是通过使用 `query` 或 `mutation` 包装器来定义的。

查询会接收一个实现了 [GenericDatabaseReader](../interfaces/server.GenericDatabaseReader.md) 接口的 `db`。

```js
import { query } from "./_generated/server";

export default query({
  handler: async ({ db }, { arg1, arg2 }) => {
    // 你的(只读)代码在这里!
  },
});
```

如果你的函数需要向数据库写入数据，比如插入、更新或删除文档，请改用 `mutation`，它会提供一个实现了 [GenericDatabaseWriter](../interfaces/server.GenericDatabaseWriter.md) 接口的 `db`。

```js
import { mutation } from "./_generated/server";

export default mutation({
  handler: async ({ db }, { arg1, arg2 }) => {
    // 在此处编写你的变更代码!
  },
});
```

## 类 \{#classes\}

* [Crons](../classes/server.Crons.md)
* [Expression](../classes/server.Expression.md)
* [IndexRange](../classes/server.IndexRange.md)
* [HttpRouter](../classes/server.HttpRouter.md)
* [TableDefinition](../classes/server.TableDefinition.md)
* [SchemaDefinition](../classes/server.SchemaDefinition.md)
* [SearchFilter](../classes/server.SearchFilter.md)
* [FilterExpression](../classes/server.FilterExpression.md)

## 接口 \{#interfaces\}

* [UserIdentity](../interfaces/server.UserIdentity.md)
* [Auth](../interfaces/server.Auth.md)
* [CronJob](../interfaces/server.CronJob.md)
* [BaseTableReader](../interfaces/server.BaseTableReader.md)
* [GenericDatabaseReader](../interfaces/server.GenericDatabaseReader.md)
* [GenericDatabaseReaderWithTable](../interfaces/server.GenericDatabaseReaderWithTable.md)
* [GenericDatabaseWriter](../interfaces/server.GenericDatabaseWriter.md)
* [GenericDatabaseWriterWithTable](../interfaces/server.GenericDatabaseWriterWithTable.md)
* [BaseTableWriter](../interfaces/server.BaseTableWriter.md)
* [FilterBuilder](../interfaces/server.FilterBuilder.md)
* [IndexRangeBuilder](../interfaces/server.IndexRangeBuilder.md)
* [PaginationResult](../interfaces/server.PaginationResult.md)
* [PaginationOptions](../interfaces/server.PaginationOptions.md)
* [QueryInitializer](../interfaces/server.QueryInitializer.md)
* [Query](../interfaces/server.Query.md)
* [OrderedQuery](../interfaces/server.OrderedQuery.md)
* [GenericMutationCtx](../interfaces/server.GenericMutationCtx.md)
* [GenericQueryCtx](../interfaces/server.GenericQueryCtx.md)
* [GenericActionCtx](../interfaces/server.GenericActionCtx.md)
* [ValidatedFunction](../interfaces/server.ValidatedFunction.md)
* [Scheduler](../interfaces/server.Scheduler.md)
* [SearchIndexConfig](../interfaces/server.SearchIndexConfig.md)
* [VectorIndexConfig](../interfaces/server.VectorIndexConfig.md)
* [DefineSchemaOptions](../interfaces/server.DefineSchemaOptions.md)
* [SystemDataModel](../interfaces/server.SystemDataModel.md)
* [SearchFilterBuilder](../interfaces/server.SearchFilterBuilder.md)
* [SearchFilterFinalizer](../interfaces/server.SearchFilterFinalizer.md)
* [StorageReader](../interfaces/server.StorageReader.md)
* [StorageWriter](../interfaces/server.StorageWriter.md)
* [StorageActionWriter](../interfaces/server.StorageActionWriter.md)
* [VectorSearchQuery](../interfaces/server.VectorSearchQuery.md)
* [VectorFilterBuilder](../interfaces/server.VectorFilterBuilder.md)

## 参考 \{#references\}

### UserIdentityAttributes \{#useridentityattributes\}

再导出 [UserIdentityAttributes](browser.md#useridentityattributes)

## 类型别名 \{#type-aliases\}

### FunctionType \{#functiontype\}

Ƭ **FunctionType**: `"query"` | `"mutation"` | `"action"`

Convex 函数的类型。

#### 定义于 \{#defined-in\}

[server/api.ts:19](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L19)

***

### FunctionReference \{#functionreference\}

Ƭ **FunctionReference**&lt;`Type`, `Visibility`, `Args`, `ReturnType`, `ComponentPath`&gt;: `Object`

对某个已注册 Convex 函数的引用。

你可以使用生成的 `api` 实用工具创建一个 [FunctionReference](server.md#functionreference)：

```js
import { api } from "../convex/_generated/api";

const reference = api.myModule.myFunction;
```

如果你没有使用代码生成，可以使用
[anyApi](server.md#anyapi-1)
来创建引用：

```js
import { anyApi } from "convex/server";

const reference = anyApi.myModule.myFunction;
```

函数引用可用于在客户端调用函数。比如，在 React 中你可以将这些引用传递给 [useQuery](react.md#usequery) hook：

```js
const result = useQuery(api.myModule.myFunction);
```

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `Type` | extends [`FunctionType`](server.md#functiontype) | 函数的类型（&quot;query&quot;、&quot;mutation&quot; 或 &quot;action&quot;）。 |
| `Visibility` | extends [`FunctionVisibility`](server.md#functionvisibility) = `"public"` | 函数的可见性（&quot;public&quot; 或 &quot;internal&quot;）。 |
| `Args` | extends [`DefaultFunctionArgs`](server.md#defaultfunctionargs) = `any` | 此函数的参数。这是一个对象，用于将参数名称映射到其类型。 |
| `ReturnType` | `any` | 此函数的返回类型。 |
| `ComponentPath` | `string` | `undefined` | - |

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 |
| :------ | :------ |
| `_type` | `Type` |
| `_visibility` | `Visibility` |
| `_args` | `Args` |
| `_returnType` | `ReturnType` |
| `_componentPath` | `ComponentPath` |

#### 定义于 \{#defined-in\}

[server/api.ts:52](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L52)

***

### ApiFromModules \{#apifrommodules\}

Ƭ **ApiFromModules**&lt;`AllModules`&gt;: [`FilterApi`](server.md#filterapi)&lt;`ApiFromModulesAllowEmptyNodes`&lt;`AllModules`&gt;, [`FunctionReference`](server.md#functionreference)&lt;`any`, `any`, `any`, `any`&gt;&gt;

给定 `convex/` 目录中所有模块的类型，构造 `api` 的类型。

`api` 是一个用于构造 [FunctionReference](server.md#functionreference) 的实用工具。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `AllModules` | extends `Record`&lt;`string`, `object`&gt; | 一个类型，用于将模块路径（例如 `"dir/myModule"`）映射到对应模块的类型。 |

#### 定义于 \{#defined-in\}

[server/api.ts:255](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L255)

***

### FilterApi \{#filterapi\}

Ƭ **FilterApi**&lt;`API`, `Predicate`&gt;: [`Expand`](server.md#expand)&lt;&#123; [mod in keyof API as API[mod] extends Predicate ? mod : API[mod] extends FunctionReference&lt;any, any, any, any&gt; ? never : FilterApi&lt;API[mod], Predicate&gt; extends Record&lt;string, never&gt; ? never : mod]: API[mod] extends Predicate ? API[mod] : FilterApi&lt;API[mod], Predicate&gt; &#125;&gt;

在 Convex 部署的 API 对象中筛选出满足给定条件的函数，
例如所有 public 查询函数。

#### 类型参数 \{#type-parameters\}

| 名称 |
| :------ |
| `API` |
| `Predicate` |

#### 定义于 \{#defined-in\}

[server/api.ts:279](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L279)

***

### AnyApi \{#anyapi\}

Ƭ **AnyApi**: `Record`&lt;`string`, `Record`&lt;`string`, `AnyModuleDirOrFunc`&gt;&gt;

Convex API 对象所扩展的类型。如果你从头开始编写一个 API，它应该扩展这个类型。

#### 定义于 \{#defined-in\}

[server/api.ts:393](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L393)

***

### PartialApi \{#partialapi\}

Ƭ **PartialApi**&lt;`API`&gt;: &#123; [mod in keyof API]?: API[mod] extends FunctionReference&lt;any, any, any, any&gt; ? API[mod] : PartialApi&lt;API[mod]&gt; &#125;

递归的部分 API 类型，在进行 mock 或构建自定义 API 对象时，用于定义某个 API 的子集。

#### 类型参数 \{#type-parameters\}

| 名称 |
| :------ |
| `API` |

#### 定义于 \{#defined-in\}

[server/api.ts:401](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L401)

***

### FunctionArgs \{#functionargs\}

Ƭ **FunctionArgs**&lt;`FuncRef`&gt;: `FuncRef`[`"_args"`]

给定一个 [FunctionReference](server.md#functionreference)，获取该函数的参数类型。

它表示为一个对象，将参数名称映射到对应的值。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `FuncRef` | extends `AnyFunctionReference` |

#### 定义于 \{#defined-in\}

[server/api.ts:435](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L435)

***

### OptionalRestArgs \{#optionalrestargs\}

Ƭ **OptionalRestArgs**&lt;`FuncRef`&gt;: `FuncRef`[`"_args"`] extends `EmptyObject` ? [args?: EmptyObject] : [args: FuncRef[&quot;&#95;args&quot;]]

一个元组类型，表示 `FuncRef` 的参数（参数本身可能是可选的）。

此类型用于让涉及参数的方法具有类型安全性，同时对于不需要参数的函数，仍然允许省略参数。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `FuncRef` | extends `AnyFunctionReference` |

#### 定义于 \{#defined-in\}

[server/api.ts:446](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L446)

***

### ArgsAndOptions \{#argsandoptions\}

Ƭ **ArgsAndOptions**&lt;`FuncRef`, `Options`&gt;: `FuncRef`[`"_args"`] extends `EmptyObject` ? [args?: EmptyObject, options?: Options] : [args: FuncRef[&quot;&#95;args&quot;], options?: Options]

一个元组类型，第一项是传给 `FuncRef` 的（可能是可选的）参数，第二项是类型为 `Options` 的选项对象。

此类型用于在保证类型安全的前提下，使像 `useQuery` 这样的方法可以：

1. 对于不需要参数的函数省略参数。
2. 省略选项对象。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `FuncRef` | extends `AnyFunctionReference` |
| `Options` | `Options` |

#### 定义于 \{#defined-in\}

[server/api.ts:460](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L460)

***

### FunctionReturnType \{#functionreturntype\}

Ƭ **FunctionReturnType**&lt;`FuncRef`&gt;: `FuncRef`[`"_returnType"`]

给定一个 [FunctionReference](server.md#functionreference)，获取其返回类型。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `FuncRef` | extends `AnyFunctionReference` |

#### 定义于 \{#defined-in\}

[server/api.ts:472](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L472)

***

### AuthConfig \{#authconfig\}

Ƭ **AuthConfig**: `Object`

由你的 Convex 项目从 `auth.config.ts` 导出的值。

```ts
import { AuthConfig } from "convex/server";

export default {
  providers: [
    {
      domain: "https://your.issuer.url.com",
      applicationID: "your-application-id",
    },
  ],
} satisfies AuthConfig;
```

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 |
| :------ | :------ |
| `providers` | [`AuthProvider`](server.md#authprovider)[] |

#### 定义于 \{#defined-in\}

[server/authentication.ts:19](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L19)

***

### AuthProvider \{#authprovider\}

Ƭ **AuthProvider**: &#123; `applicationID`: `string` ; `domain`: `string`  &#125; | &#123; `type`: `"customJwt"` ; `applicationID?`: `string` ; `issuer`: `string` ; `jwks`: `string` ; `algorithm`: `"RS256"` | `"ES256"`  &#125;

一个被授权为你的应用签发 JWT 的身份验证提供方。

参见：https://docs.convex.dev/auth/advanced/custom-auth 和 https://docs.convex.dev/auth/advanced/custom-jwt

#### 定义于 \{#defined-in\}

[server/authentication.ts:28](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L28)

***

### FunctionHandle \{#functionhandle\}

Ƭ **FunctionHandle**&lt;`Type`, `Args`, `ReturnType`&gt;: `string` &amp; [`FunctionReference`](server.md#functionreference)&lt;`Type`, `"internal"`, `Args`, `ReturnType`&gt;

对 Convex 函数的可序列化引用。
将此引用传递给另一个组件，可以让该组件在当前函数执行期间或未来任意时刻调用这个函数。
函数句柄的使用方式类似于 `api.folder.function` 的函数引用（FunctionReference），
例如 `ctx.scheduler.runAfter(0, functionReference, args)`。

函数引用在代码推送之间是稳定的，但它所引用的 Convex 函数有可能已经不存在。

这是组件的一项特性，目前处于 beta 阶段。
此 API 尚不稳定，在后续版本中可能会发生变化。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | extends [`FunctionType`](server.md#functiontype) |
| `Args` | extends [`DefaultFunctionArgs`](server.md#defaultfunctionargs) = `any` |
| `ReturnType` | `any` |

#### 定义于 \{#defined-in\}

[server/components/index.ts:35](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L35)

***

### ComponentDefinition \{#componentdefinition\}

Ƭ **ComponentDefinition**&lt;`Exports`&gt;: `Object`

此类型的对象应作为组件定义目录中 convex.config.ts 文件的默认导出。

这是组件的一个特性，目前处于 beta 阶段。
该 API 尚不稳定，后续版本中可能会发生变更。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Exports` | extends `ComponentExports` = `any` |

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `use` | &lt;Definition&gt;(`definition`: `Definition`, `options?`: &#123; `name?`: `string`  &#125;) =&gt; `InstalledComponent`&lt;`Definition`&gt; | 在此组件定义中安装具有给定定义的组件。接收一个组件定义以及一个可选名称。对于编辑器工具，此方法需要一个 [ComponentDefinition](server.md#componentdefinition)，但在运行时实际被导入的对象将是一个 ImportedComponentDefinition。 |
| `__exports` | `Exports` | 仅限类型的内部属性，用于跟踪所提供的导出。**`已弃用`** 这是一个仅用于类型的属性，请不要使用。 |

#### 定义于 \{#defined-in\}

[server/components/index.ts:84](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L84)

***

### AnyChildComponents \{#anychildcomponents\}

Ƭ **AnyChildComponents**: `Record`&lt;`string`, `AnyComponentReference`&gt;

#### 定义于 \{#defined-in\}

[server/components/index.ts:414](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L414)

***

### AnyComponents \{#anycomponents\}

Ƭ **AnyComponents**: [`AnyChildComponents`](server.md#anychildcomponents)

#### 定义于 \{#defined-in\}

[server/components/index.ts:454](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L454)

***

### GenericDocument \{#genericdocument\}

Ƭ **GenericDocument**: `Record`&lt;`string`, [`Value`](values.md#value)&gt;

在 Convex 中存储的文档。

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:9](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L9)

***

### GenericFieldPaths \{#genericfieldpaths\}

Ƭ **GenericFieldPaths**: `string`

一个用于描述表中所有文档字段的类型。

它可以是字段名（例如 &quot;name&quot;），也可以是对嵌套对象字段的引用（例如 &quot;properties.name&quot;）。

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:18](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L18)

***

### GenericIndexFields \{#genericindexfields\}

Ƭ **GenericIndexFields**: `string`[]

用于描述索引中字段顺序的类型。

这些字段可以是字段名（例如 &quot;name&quot;），也可以是嵌套对象上字段的引用（例如 &quot;properties.name&quot;）。

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:29](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L29)

***

### GenericTableIndexes \{#generictableindexes\}

Ƭ **GenericTableIndexes**: `Record`&lt;`string`, [`GenericIndexFields`](server.md#genericindexfields)&gt;

用于描述表中索引的类型。

这是一个对象，将每个索引名称映射到该索引所包含的字段。

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:37](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L37)

***

### GenericSearchIndexConfig \{#genericsearchindexconfig\}

Ƭ **GenericSearchIndexConfig**: `Object`

用于描述搜索索引配置的类型。

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 |
| :------ | :------ |
| `searchField` | `string` |
| `filterFields` | `string` |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:43](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L43)

***

### GenericTableSearchIndexes \{#generictablesearchindexes\}

Ƭ **GenericTableSearchIndexes**: `Record`&lt;`string`, [`GenericSearchIndexConfig`](server.md#genericsearchindexconfig)&gt;

一个用于描述表中所有搜索索引的类型。

这是一个对象，将每个索引名称映射到对应的索引配置。

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:54](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L54)

***

### GenericVectorIndexConfig \{#genericvectorindexconfig\}

Ƭ **GenericVectorIndexConfig**: `Object`

一个描述向量索引配置的类型。

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 |
| :------ | :------ |
| `vectorField` | `string` |
| `dimensions` | `number` |
| `filterFields` | `string` |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:63](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L63)

***

### GenericTableVectorIndexes \{#generictablevectorindexes\}

Ƭ **GenericTableVectorIndexes**: `Record`&lt;`string`, [`GenericVectorIndexConfig`](server.md#genericvectorindexconfig)&gt;

一个用于描述表中所有向量索引的类型。

这是一个对象，将每个索引名称映射到对应索引的配置。

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:75](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L75)

***

### FieldTypeFromFieldPath \{#fieldtypefromfieldpath\}

Ƭ **FieldTypeFromFieldPath**&lt;`Document`, `FieldPath`&gt;: [`FieldTypeFromFieldPathInner`](server.md#fieldtypefromfieldpathinner)&lt;`Document`, `FieldPath`&gt; extends [`Value`](values.md#value) | `undefined` ? [`FieldTypeFromFieldPathInner`](server.md#fieldtypefromfieldpathinner)&lt;`Document`, `FieldPath`&gt; : [`Value`](values.md#value) | `undefined`

该类型表示文档中某个字段的类型。

注意，这同时支持像 &quot;name&quot; 这样的简单字段，以及像
&quot;properties.name&quot; 这样的嵌套字段。

如果该字段在文档中不存在，则被视为 `undefined`。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Document` | 约束为 [`GenericDocument`](server.md#genericdocument) 的子类型 |
| `FieldPath` | 约束为 `string` 的子类型 |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:104](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L104)

***

### FieldTypeFromFieldPathInner \{#fieldtypefromfieldpathinner\}

Ƭ **FieldTypeFromFieldPathInner**&lt;`Document`, `FieldPath`&gt;: `FieldPath` extends `$&#123;infer First&#125;.$&#123;infer Second&#125;` ? `ValueFromUnion`&lt;`Document`, `First`, `Record`&lt;`never`, `never`&gt;&gt; extends infer FieldValue ? `FieldValue` extends [`GenericDocument`](server.md#genericdocument) ? [`FieldTypeFromFieldPath`](server.md#fieldtypefromfieldpath)&lt;`FieldValue`, `Second`&gt; : `undefined` : `undefined` : `ValueFromUnion`&lt;`Document`, `FieldPath`, `undefined`&gt;

[FieldTypeFromFieldPath](server.md#fieldtypefromfieldpath) 的内部类型。

它被包装在一个辅助类型中，用于将该类型强制为 `Value | undefined`，因为某些
版本的 TypeScript 无法正确推断该类型。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Document` | 扩展自 [`GenericDocument`](server.md#genericdocument) |
| `FieldPath` | 扩展自 `string` |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:120](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L120)

***

### GenericTableInfo \{#generictableinfo\}

Ƭ **GenericTableInfo**: `Object`

用于描述表中文档类型和索引的类型别名。

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 |
| :------ | :------ |
| `document` | [`GenericDocument`](server.md#genericdocument) |
| `fieldPaths` | [`GenericFieldPaths`](server.md#genericfieldpaths) |
| `indexes` | [`GenericTableIndexes`](server.md#generictableindexes) |
| `searchIndexes` | [`GenericTableSearchIndexes`](server.md#generictablesearchindexes) |
| `vectorIndexes` | [`GenericTableVectorIndexes`](server.md#generictablevectorindexes) |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:145](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L145)

***

### DocumentByInfo \{#documentbyinfo\}

Ƭ **DocumentByInfo**&lt;`TableInfo`&gt;: `TableInfo`[`"document"`]

给定某个 [GenericTableInfo](server.md#generictableinfo) 时，对应表中文档的类型。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableInfo` | extends [`GenericTableInfo`](server.md#generictableinfo) |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:157](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L157)

***

### FieldPaths \{#fieldpaths\}

Ƭ **FieldPaths**&lt;`TableInfo`&gt;: `TableInfo`[`"fieldPaths"`]

对于给定的 [GenericTableInfo](server.md#generictableinfo)，表中的字段路径。

这些可以是字段名（例如 &quot;name&quot;），也可以是嵌套对象中字段的引用（例如 &quot;properties.name&quot;）。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableInfo` | 扩展自 [`GenericTableInfo`](server.md#generictableinfo) |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:167](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L167)

***

### Indexes \{#indexes\}

Ƭ **Indexes**&lt;`TableInfo`&gt;: `TableInfo`[`"indexes"`]

对于给定的 [GenericTableInfo](server.md#generictableinfo)，表示表中的数据库索引。

这是一个对象，将索引名称映射到该索引包含的字段。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableInfo` | 继承自 [`GenericTableInfo`](server.md#generictableinfo) |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:176](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L176)

***

### IndexNames \{#indexnames\}

Ƭ **IndexNames**&lt;`TableInfo`&gt;: keyof [`Indexes`](server.md#indexes)&lt;`TableInfo`&gt;

对于给定的 [GenericTableInfo](server.md#generictableinfo)，某个表中索引的名称。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableInfo` | 继承自 [`GenericTableInfo`](server.md#generictableinfo) |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:182](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L182)

***

### NamedIndex \{#namedindex\}

Ƭ **NamedIndex**&lt;`TableInfo`, `IndexName`&gt;: [`Indexes`](server.md#indexes)&lt;`TableInfo`&gt;[`IndexName`]

根据名称从 [GenericTableInfo](server.md#generictableinfo) 中提取指定索引的字段。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableInfo` | 受限于 [`GenericTableInfo`](server.md#generictableinfo) |
| `IndexName` | 受限于 [`IndexNames`](server.md#indexnames)&lt;`TableInfo`&gt; |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:189](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L189)

***

### SearchIndexes \{#searchindexes\}

Ƭ **SearchIndexes**&lt;`TableInfo`&gt;: `TableInfo`[`"searchIndexes"`]

给定某个 [GenericTableInfo](server.md#generictableinfo) 时，表示表中的搜索索引集合。

这是一个对象，将索引名称映射到对应的搜索索引配置。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableInfo` | 继承自 [`GenericTableInfo`](server.md#generictableinfo) |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:200](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L200)

***

### SearchIndexNames \{#searchindexnames\}

Ƭ **SearchIndexNames**&lt;`TableInfo`&gt;: keyof [`SearchIndexes`](server.md#searchindexes)&lt;`TableInfo`&gt;

对于指定的 [GenericTableInfo](server.md#generictableinfo)，表中所有搜索索引的名称。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableInfo` | 继承自 [`GenericTableInfo`](server.md#generictableinfo) |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:207](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L207)

***

### NamedSearchIndex \{#namedsearchindex\}

Ƭ **NamedSearchIndex**&lt;`TableInfo`, `IndexName`&gt;: [`SearchIndexes`](server.md#searchindexes)&lt;`TableInfo`&gt;[`IndexName`]

通过名称从 [GenericTableInfo](server.md#generictableinfo) 中提取指定搜索索引的配置。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableInfo` | 扩展自 [`GenericTableInfo`](server.md#generictableinfo) |
| `IndexName` | 扩展自 [`SearchIndexNames`](server.md#searchindexnames)&lt;`TableInfo`&gt; |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:214](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L214)

***

### VectorIndexes \{#vectorindexes\}

Ƭ **VectorIndexes**&lt;`TableInfo`&gt;: `TableInfo`[`"vectorIndexes"`]

对于给定的 [GenericTableInfo](server.md#generictableinfo)，表示表中的向量索引。

这是一个对象，用于将索引名称映射到相应的向量索引配置。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableInfo` | 需扩展自 [`GenericTableInfo`](server.md#generictableinfo) |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:225](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L225)

***

### VectorIndexNames \{#vectorindexnames\}

Ƭ **VectorIndexNames**&lt;`TableInfo`&gt;: keyof [`VectorIndexes`](server.md#vectorindexes)&lt;`TableInfo`&gt;

对于给定的 [GenericTableInfo](server.md#generictableinfo)，某张表中所有向量索引的名称。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableInfo` | 扩展自 [`GenericTableInfo`](server.md#generictableinfo) |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:232](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L232)

***

### NamedVectorIndex \{#namedvectorindex\}

Ƭ **NamedVectorIndex**&lt;`TableInfo`, `IndexName`&gt;: [`VectorIndexes`](server.md#vectorindexes)&lt;`TableInfo`&gt;[`IndexName`]

通过名称从 [GenericTableInfo](server.md#generictableinfo) 中提取某个向量索引的配置。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableInfo` | 扩展自 [`GenericTableInfo`](server.md#generictableinfo) |
| `IndexName` | 扩展自 [`VectorIndexNames`](server.md#vectorindexnames)&lt;`TableInfo`&gt; |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:239](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L239)

***

### GenericDataModel \{#genericdatamodel\}

Ƭ **GenericDataModel**: `Record`&lt;`string`, [`GenericTableInfo`](server.md#generictableinfo)&gt;

一个用于描述 Convex 项目中各数据表的类型。

该类型旨在通过 `npx convex dev` 进行代码生成。

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:252](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L252)

***

### AnyDataModel \{#anydatamodel\}

Ƭ **AnyDataModel**: `Object`

一种 [GenericDataModel](server.md#genericdatamodel)，将文档视为 `any` 类型且不支持索引。

在定义模式之前，这是默认的数据模型。

#### 索引签名 \{#index-signature\}

▪ [tableName: `string`]: &#123; `document`: `any` ; `fieldPaths`: [`GenericFieldPaths`](server.md#genericfieldpaths) ; `indexes`: {} ; `searchIndexes`: {} ; `vectorIndexes`: {}  &#125;

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:261](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L261)

***

### TableNamesInDataModel \{#tablenamesindatamodel\}

Ƭ **TableNamesInDataModel**&lt;`DataModel`&gt;: keyof `DataModel` &amp; `string`

表示在 [GenericDataModel](server.md#genericdatamodel) 中定义的所有表名的类型。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | 扩展自 [`GenericDataModel`](server.md#genericdatamodel) |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:275](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L275)

***

### NamedTableInfo \{#namedtableinfo\}

Ƭ **NamedTableInfo**&lt;`DataModel`, `TableName`&gt;: `DataModel`[`TableName`]

根据表名从 [GenericDataModel](server.md#genericdatamodel) 中提取指定表的 `TableInfo`。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](server.md#genericdatamodel) |
| `TableName` | extends keyof `DataModel` |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:284](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L284)

***

### DocumentByName \{#documentbyname\}

Ƭ **DocumentByName**&lt;`DataModel`, `TableName`&gt;: `DataModel`[`TableName`][`"document"`]

在 [GenericDataModel](server.md#genericdatamodel) 中按表名对应的文档类型。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](server.md#genericdatamodel) |
| `TableName` | extends [`TableNamesInDataModel`](server.md#tablenamesindatamodel)&lt;`DataModel`&gt; |

#### 定义于 \{#defined-in\}

[server/data&#95;model.ts:293](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L293)

***

### ExpressionOrValue \{#expressionorvalue\}

Ƭ **ExpressionOrValue**&lt;`T`&gt;: [`Expression`](../classes/server.Expression.md)&lt;`T`&gt; | `T`

一个 [`Expression`](../classes/server.Expression.md) 或常量 [`值`](values.md#value)

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | extends [`Value`](values.md#value) | `undefined` |

#### 定义于 \{#defined-in\}

[server/filter&#95;builder.ts:38](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L38)

***

### Cursor \{#cursor\}

Ƭ **Cursor**: `string`

用于对数据库查询进行分页的不透明标识符。

Cursor 由 [paginate](../interfaces/server.OrderedQuery.md#paginate) 返回，并表示该次查询中结果页结束的位置。

要继续分页，将 Cursor 重新传入
[PaginationOptions](../interfaces/server.PaginationOptions.md) 对象中的
[paginate](../interfaces/server.OrderedQuery.md#paginate)，以获取下一页结果。

注意：Cursor 只能传给与其生成时 *完全* 相同的数据库查询。你不能在不同的
数据库查询之间复用同一个 Cursor。

#### 定义于 \{#defined-in\}

[server/pagination.ts:21](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L21)

***

### GenericMutationCtxWithTable \{#genericmutationctxwithtable\}

Ƭ **GenericMutationCtxWithTable**&lt;`DataModel`&gt;: `Omit`&lt;[`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`DataModel`&gt;, `"db"`&gt; &amp; &#123; `db`: [`GenericDatabaseWriterWithTable`](../interfaces/server.GenericDatabaseWriterWithTable.md)&lt;`DataModel`&gt;  &#125;

一组可在 Convex 变更函数中使用的服务。

变更上下文会作为第一个参数传递给在服务器上运行的任何 Convex 变更函数。

如果你在使用代码生成，请在 `convex/_generated/server.d.ts` 中使用已根据你的数据模型完成类型定义的 `MutationCtx` 类型。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | 扩展自 [`GenericDataModel`](server.md#genericdatamodel) |

#### 定义于 \{#defined-in\}

[server/registration.ts:109](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L109)

***

### GenericQueryCtxWithTable \{#genericqueryctxwithtable\}

Ƭ **GenericQueryCtxWithTable**&lt;`DataModel`&gt;: `Omit`&lt;[`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`DataModel`&gt;, `"db"`&gt; &amp; &#123; `db`: [`GenericDatabaseReaderWithTable`](../interfaces/server.GenericDatabaseReaderWithTable.md)&lt;`DataModel`&gt;  &#125;

一组可在 Convex 查询函数中使用的服务。

查询上下文会作为第一个参数传递给在服务器上运行的任何 Convex 查询函数。

它与 `MutationCtx` 不同，因为其中所有服务都是只读的。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | 扩展自 [`GenericDataModel`](server.md#genericdatamodel) |

#### 定义于 \{#defined-in\}

[server/registration.ts:167](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L167)

***

### DefaultFunctionArgs \{#defaultfunctionargs\}

Ƭ **DefaultFunctionArgs**: `Record`&lt;`string`, `unknown`&gt;

Convex 查询、变更或操作函数的默认参数类型。

Convex 函数始终接收一个参数对象，该对象将参数名映射到对应的值。

#### 定义于 \{#defined-in\}

[server/registration.ts:278](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L278)

***

### ArgsArray \{#argsarray\}

Ƭ **ArgsArray**: `OneArgArray` | `NoArgsArray`

传递给 Convex 函数的参数数组。

Convex 函数可以接收一个 [DefaultFunctionArgs](server.md#defaultfunctionargs) 对象，也可以不接收任何参数。

#### 定义于 \{#defined-in\}

[server/registration.ts:301](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L301)

***

### ArgsArrayToObject \{#argsarraytoobject\}

Ƭ **ArgsArrayToObject**&lt;`Args`&gt;: `Args` extends `OneArgArray`&lt;infer ArgsObject&gt; ? `ArgsObject` : `EmptyObject`

将 [ArgsArray](server.md#argsarray) 转换为单个对象类型。

空参数数组将被转换为 EmptyObject。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Args` | 扩展自 [`ArgsArray`](server.md#argsarray) |

#### 定义于 \{#defined-in\}

[server/registration.ts:316](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L316)

***

### FunctionVisibility \{#functionvisibility\}

Ƭ **FunctionVisibility**: `"public"` | `"internal"`

用于表示 Convex 函数可见性的类型。

#### 定义于 \{#defined-in\}

[server/registration.ts:324](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L324)

***

### RegisteredMutation \{#registeredmutation\}

Ƭ **RegisteredMutation**&lt;`Visibility`, `Args`, `Returns`&gt;: &#123; `isConvexFunction`: `true` ; `isMutation`: `true`  &#125; &amp; `VisibilityProperties`&lt;`Visibility`&gt;

此应用中的一个变更函数。

你可以通过将函数包裹在
[mutationGeneric](server.md#mutationgeneric) 或 [internalMutationGeneric](server.md#internalmutationgeneric) 中并导出来创建一个变更函数。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Visibility` | 继承自 [`FunctionVisibility`](server.md#functionvisibility) |
| `Args` | 继承自 [`DefaultFunctionArgs`](server.md#defaultfunctionargs) |
| `Returns` | `Returns` |

#### 定义位置 \{#defined-in\}

[server/registration.ts:347](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L347)

***

### RegisteredQuery \{#registeredquery\}

Ƭ **RegisteredQuery**&lt;`Visibility`, `Args`, `Returns`&gt;: &#123; `isConvexFunction`: `true` ; `isQuery`: `true`  &#125; &amp; `VisibilityProperties`&lt;`Visibility`&gt;

作为此应用一部分的查询函数。

可以通过将函数包装在
[queryGeneric](server.md#querygeneric) 或 [internalQueryGeneric](server.md#internalquerygeneric) 中并将其导出，来创建一个查询。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Visibility` | 扩展自 [`FunctionVisibility`](server.md#functionvisibility) |
| `Args` | 扩展自 [`DefaultFunctionArgs`](server.md#defaultfunctionargs) |
| `Returns` | `Returns` |

#### 定义于 \{#defined-in\}

[server/registration.ts:376](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L376)

***

### RegisteredAction \{#registeredaction\}

Ƭ **RegisteredAction**&lt;`Visibility`, `Args`, `Returns`&gt;: &#123; `isConvexFunction`: `true` ; `isAction`: `true`  &#125; &amp; `VisibilityProperties`&lt;`Visibility`&gt;

作为该应用一部分的操作。

你可以将函数包装在 [actionGeneric](server.md#actiongeneric) 或 [internalActionGeneric](server.md#internalactiongeneric) 中并导出，从而创建一个操作。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Visibility` | 继承自 [`FunctionVisibility`](server.md#functionvisibility) |
| `Args` | 继承自 [`DefaultFunctionArgs`](server.md#defaultfunctionargs) |
| `Returns` | `Returns` |

#### 定义于 \{#defined-in\}

[server/registration.ts:405](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L405)

***

### PublicHttpAction \{#publichttpaction\}

Ƭ **PublicHttpAction**: `Object`

作为此应用公共 API 一部分的 HTTP 操作函数。

可以通过将函数包装在
[httpActionGeneric](server.md#httpactiongeneric) 中并导出它来创建公共 HTTP 操作函数。

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 |
| :------ | :------ |
| `isHttp` | `true` |

#### 定义于 \{#defined-in\}

[server/registration.ts:434](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L434)

***

### UnvalidatedFunction \{#unvalidatedfunction\}

Ƭ **UnvalidatedFunction**&lt;`Ctx`, `Args`, `Returns`&gt;: (`ctx`: `Ctx`, ...`args`: `Args`) =&gt; `Returns` | &#123; `handler`: (`ctx`: `Ctx`, ...`args`: `Args`) =&gt; `Returns`  &#125;

**`Deprecated`**

—— 请参阅 `MutationBuilder` 或类似类型的定义，
了解用于定义 Convex 函数的相关类型。

用于在定义 Convex 查询、变更或操作函数时不进行参数验证的类型。

Convex 函数总是以上下文对象作为第一个参数，
并以（可选的）`args` 对象作为第二个参数。

这可以写成如下形式的函数：

```js
import { query } from "./_generated/server";

export const func = query(({ db }, { arg }) => {...});
```

或者作为对象，例如：

```js
import { query } from "./_generated/server";

export const func = query({
  handler: ({ db }, { arg }) => {...},
});
```

请参阅 [ValidatedFunction](../interfaces/server.ValidatedFunction.md) 来添加参数验证。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Ctx` | `Ctx` |
| `Args` | 扩展自 [`ArgsArray`](server.md#argsarray) |
| `Returns` | `Returns` |

#### 定义于 \{#defined-in\}

[server/registration.ts:472](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L472)

***

### ReturnValueForOptionalValidator \{#returnvalueforoptionalvalidator\}

Ƭ **ReturnValueForOptionalValidator**&lt;`ReturnsValidator`&gt;: [`ReturnsValidator`] extends [[`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt;] ? `ValidatorTypeToReturnType`&lt;[`Infer`](values.md#infer)&lt;`ReturnsValidator`&gt;&gt; : [`ReturnsValidator`] extends [[`PropertyValidators`](values.md#propertyvalidators)] ? `ValidatorTypeToReturnType`&lt;[`ObjectType`](values.md#objecttype)&lt;`ReturnsValidator`&gt;&gt; : `any`

可以使用多种语法来定义 Convex 函数：

```
 - query(async (ctx, args) => {...})
 - query({ handler: async (ctx, args) => {...} })
 - query({ args: { a: v.string }, handler: async (ctx, args) => {...} } })
 - query({ args: { a: v.string }, returns: v.string(), handler: async (ctx, args) => {...} } })
```

在这些场景中，我们都希望能够正确推断参数和返回值的类型，如果提供了 validator，则优先使用从 validator 推断出的类型。

为了避免为每种情况单独编写重载（这些重载会出现在错误消息中），
我们使用类型参数——ArgsValidator、ReturnsValidator、ReturnValue、OneOrZeroArgs。

ReturnValue 和 OneOrZeroArgs 的类型会被 ArgsValidator 和 ReturnsValidator 的类型所约束（如果它们存在），
并且还会从函数参数或返回值上的显式类型注解中推断。

下面是一些工具类型，用于在可选 validator 的基础上获取合适的类型约束。

一些额外的技巧：

* 我们使用 Validator | void 而不是 Validator | undefined，因为在 `strictNullChecks` 下，
  后者等价于 `Validator`，不起作用。
* 我们使用长度为 1 的元组类型来避免在联合类型上发生分布
  https://github.com/microsoft/TypeScript/issues/29368#issuecomment-453529532

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `ReturnsValidator` | extends [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) | `void` |

#### 定义于 \{#defined-in\}

[server/registration.ts:574](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L574)

***

### ArgsArrayForOptionalValidator \{#argsarrayforoptionalvalidator\}

Ƭ **ArgsArrayForOptionalValidator**&lt;`ArgsValidator`&gt;: [`ArgsValidator`] extends [[`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt;] ? `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; : [`ArgsValidator`] extends [[`PropertyValidators`](values.md#propertyvalidators)] ? `OneArgArray`&lt;[`ObjectType`](values.md#objecttype)&lt;`ArgsValidator`&gt;&gt; : [`ArgsArray`](server.md#argsarray)

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `ArgsValidator` | extends [`GenericValidator`](values.md#genericvalidator) | [`PropertyValidators`](values.md#propertyvalidators) | `void` |

#### 定义于 \{#defined-in\}

[server/registration.ts:582](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L582)

***

### DefaultArgsForOptionalValidator \{#defaultargsforoptionalvalidator\}

Ƭ **DefaultArgsForOptionalValidator**&lt;`ArgsValidator`&gt;: [`ArgsValidator`] extends [[`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt;] ? [[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;] : [`ArgsValidator`] extends [[`PropertyValidators`](values.md#propertyvalidators)] ? [[`ObjectType`](values.md#objecttype)&lt;`ArgsValidator`&gt;] : `OneArgArray`

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `ArgsValidator` | extends [`GenericValidator`](values.md#genericvalidator) | [`PropertyValidators`](values.md#propertyvalidators) | `void` |

#### 定义于 \{#defined-in\}

[server/registration.ts:590](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L590)

***

### MutationBuilder \{#mutationbuilder\}

Ƭ **MutationBuilder**&lt;`DataModel`, `Visibility`&gt;: &lt;ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs&gt;(`mutation`: &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`) =&gt; [`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | 继承自 [`GenericDataModel`](server.md#genericdatamodel) |
| `Visibility` | 继承自 [`FunctionVisibility`](server.md#functionvisibility) |

#### 类型声明 \{#type-declaration\}

▸ &lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`mutation`): [`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

供 Convex 代码生成使用的内部类型辅助工具。

用于为 [mutationGeneric](server.md#mutationgeneric) 指定一个与你的数据模型对应的类型。

##### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

##### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `mutation` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

##### 返回 \{#returns\}

[`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 定义于 \{#defined-in\}

[server/registration.ts:604](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L604)

***

### MutationBuilderWithTable \{#mutationbuilderwithtable\}

Ƭ **MutationBuilderWithTable**&lt;`DataModel`, `Visibility`&gt;: &lt;ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs&gt;(`mutation`: &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtxWithTable`](server.md#genericmutationctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtxWithTable`](server.md#genericmutationctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`) =&gt; [`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | 扩展自 [`GenericDataModel`](server.md#genericdatamodel) |
| `Visibility` | 扩展自 [`FunctionVisibility`](server.md#functionvisibility) |

#### 类型声明 \{#type-declaration\}

▸ &lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`mutation`): [`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Convex 代码生成中使用的内部辅助类型。

用于为 [mutationGeneric](server.md#mutationgeneric) 提供一个针对你的数据模型的特定类型。

##### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

##### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `mutation` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtxWithTable`](server.md#genericmutationctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtxWithTable`](server.md#genericmutationctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

##### 返回 \{#returns\}

[`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 定义在 \{#defined-in\}

[server/registration.ts:697](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L697)

***

### QueryBuilder \{#querybuilder\}

Ƭ **QueryBuilder**&lt;`DataModel`, `Visibility`&gt;: &lt;ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs&gt;(`query`: &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`) =&gt; [`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | 继承自 [`GenericDataModel`](server.md#genericdatamodel) |
| `Visibility` | 继承自 [`FunctionVisibility`](server.md#functionvisibility) |

#### 类型声明 \{#type-declaration\}

▸ &lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`query`): [`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Convex 代码生成内部使用的类型辅助工具。

用于为 [queryGeneric](server.md#querygeneric) 提供一个特定于你的数据模型的类型。

##### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

##### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `query` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

##### 返回 \{#returns\}

[`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 定义于 \{#defined-in\}

[server/registration.ts:790](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L790)

***

### QueryBuilderWithTable \{#querybuilderwithtable\}

Ƭ **QueryBuilderWithTable**&lt;`DataModel`, `Visibility`&gt;: &lt;ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs&gt;(`query`: &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtxWithTable`](server.md#genericqueryctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtxWithTable`](server.md#genericqueryctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`) =&gt; [`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | 继承 [`GenericDataModel`](server.md#genericdatamodel) |
| `Visibility` | 继承 [`FunctionVisibility`](server.md#functionvisibility) |

#### 类型声明 \{#type-declaration\}

▸ &lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`query`): [`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

在 Convex 代码生成中使用的内部辅助类型。

用于为 [queryGeneric](server.md#querygeneric) 提供一个特定于你的数据模型的类型。

##### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

##### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `query` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtxWithTable`](server.md#genericqueryctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtxWithTable`](server.md#genericqueryctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

##### 返回 \{#returns\}

[`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 定义于 \{#defined-in\}

[server/registration.ts:879](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L879)

***

### ActionBuilder \{#actionbuilder\}

Ƭ **ActionBuilder**&lt;`DataModel`, `Visibility`&gt;: &lt;ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs&gt;(`func`: &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`) =&gt; [`RegisteredAction`](server.md#registeredaction)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | 继承自 [`GenericDataModel`](server.md#genericdatamodel) |
| `Visibility` | 继承自 [`FunctionVisibility`](server.md#functionvisibility) |

#### 类型声明 \{#type-declaration\}

▸ &lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`func`): [`RegisteredAction`](server.md#registeredaction)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

供 Convex 代码生成内部使用的类型辅助工具。

用于为 [actionGeneric](server.md#actiongeneric) 提供特定于你的数据模型的类型。

##### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

##### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `func` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

##### 返回 \{#returns\}

[`RegisteredAction`](server.md#registeredaction)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### 定义于 \{#defined-in\}

[server/registration.ts:968](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L968)

***

### HttpActionBuilder \{#httpactionbuilder\}

Ƭ **HttpActionBuilder**: (`func`: (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, `request`: `Request`) =&gt; `Promise`&lt;`Response`&gt;) =&gt; [`PublicHttpAction`](server.md#publichttpaction)

#### 类型声明 \{#type-declaration\}

▸ (`func`): [`PublicHttpAction`](server.md#publichttpaction)

在 Convex 代码生成中使用的内部类型辅助工具。

用于为 [httpActionGeneric](server.md#httpactiongeneric) 提供一个特定于你的数据模型和函数的类型。

##### 参数 \{#parameters\}

| 参数名 | 类型 |
| :------ | :------ |
| `func` | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, `request`: `Request`) =&gt; `Promise`&lt;`Response`&gt; |

##### 返回值 \{#returns\}

[`PublicHttpAction`](server.md#publichttpaction)

#### 定义于 \{#defined-in\}

[server/registration.ts:1063](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L1063)

***

### RoutableMethod \{#routablemethod\}

Ƭ **RoutableMethod**: typeof [`ROUTABLE_HTTP_METHODS`](server.md#routable_http_methods)[`number`]

表示 Convex HTTP 操作函数所支持的请求方法的类型。

HEAD 由 Convex 通过执行 GET 并去除响应体来处理。
CONNECT 不受支持，且将来也不会被支持。
TRACE 不受支持，且将来也不会被支持。

#### 定义于 \{#defined-in\}

[server/router.ts:31](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L31)

***

### RouteSpecWithPath \{#routespecwithpath\}

Ƭ **RouteSpecWithPath**: `Object`

表示使用精确请求 URL 路径匹配到 HTTP 操作的路由类型。

由 [HttpRouter](../classes/server.HttpRouter.md) 使用，用于将请求路由到 HTTP 操作函数。

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `path` | `string` | 要路由的精确 HTTP 请求路径。 |
| `method` | [`RoutableMethod`](server.md#routablemethod) | 要路由的 HTTP 请求方法（&quot;GET&quot;、&quot;POST&quot; 等）。 |
| `handler` | [`PublicHttpAction`](server.md#publichttpaction) | 要执行的 HTTP 操作。 |

#### 定义于 \{#defined-in\}

[server/router.ts:56](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L56)

***

### RouteSpecWithPathPrefix \{#routespecwithpathprefix\}

Ƭ **RouteSpecWithPathPrefix**: `Object`

表示一种路由类型，它使用请求 URL 路径前缀匹配，将请求路由到 HTTP 操作。

由 [HttpRouter](../classes/server.HttpRouter.md) 使用，用于将请求路由到 HTTP 操作函数。

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `pathPrefix` | `string` | 要路由的 HTTP 请求路径前缀。路径以该值开头的请求会被路由到该 HTTP 操作。 |
| `method` | [`RoutableMethod`](server.md#routablemethod) | 要路由的 HTTP 方法（&quot;GET&quot;、&quot;POST&quot; 等）。 |
| `handler` | [`PublicHttpAction`](server.md#publichttpaction) | 要执行的 HTTP 操作。 |

#### 定义于 \{#defined-in\}

[server/router.ts:78](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L78)

***

### RouteSpec \{#routespec\}

Ƭ **RouteSpec**: [`RouteSpecWithPath`](server.md#routespecwithpath) | [`RouteSpecWithPathPrefix`](server.md#routespecwithpathprefix)

表示用于指向某个 HTTP 操作函数的路由的类型。

由 [HttpRouter](../classes/server.HttpRouter.md) 用来将请求路由到 HTTP 操作函数。

#### 定义于 \{#defined-in\}

[server/router.ts:101](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L101)

***

### SchedulableFunctionReference \{#schedulablefunctionreference\}

Ƭ **SchedulableFunctionReference**: [`FunctionReference`](server.md#functionreference)&lt;`"mutation"` | `"action"`, `"public"` | `"internal"`&gt;

可以被安排在将来运行的 [FunctionReference](server.md#functionreference)。

可调度函数是指访问级别为 public 或 internal 的变更函数和操作函数。

#### 定义于 \{#defined-in\}

[server/scheduler.ts:11](https://github.com/get-convex/convex-js/blob/main/src/server/scheduler.ts#L11)

***

### GenericSchema \{#genericschema\}

Ƭ **GenericSchema**: `Record`&lt;`string`, [`TableDefinition`](../classes/server.TableDefinition.md)&gt;

用于描述 Convex 项目模式的类型。

应使用 [defineSchema](server.md#defineschema)、[defineTable](server.md#definetable) 和 [v](values.md#v) 来构建。

#### 定义于 \{#defined-in\}

[server/schema.ts:645](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L645)

***

### DataModelFromSchemaDefinition \{#datamodelfromschemadefinition\}

Ƭ **DataModelFromSchemaDefinition**&lt;`SchemaDef`&gt;: `MaybeMakeLooseDataModel`&lt;&#123; [TableName in keyof SchemaDef[&quot;tables&quot;] &amp; string]: SchemaDef[&quot;tables&quot;][TableName] extends TableDefinition&lt;infer DocumentType, infer Indexes, infer SearchIndexes, infer VectorIndexes&gt; ? Object : never &#125;, `SchemaDef`[`"strictTableNameTypes"`]&gt;

Convex 代码生成所使用的内部类型！

将 [SchemaDefinition](../classes/server.SchemaDefinition.md) 转换为 [GenericDataModel](server.md#genericdatamodel)。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `SchemaDef` | 扩展自 [`SchemaDefinition`](../classes/server.SchemaDefinition.md)&lt;`any`, `boolean`&gt; |

#### 定义于 \{#defined-in\}

[server/schema.ts:786](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L786)

***

### SystemTableNames \{#systemtablenames\}

Ƭ **SystemTableNames**: [`TableNamesInDataModel`](server.md#tablenamesindatamodel)&lt;[`SystemDataModel`](../interfaces/server.SystemDataModel.md)&gt;

#### 定义于 \{#defined-in\}

[server/schema.ts:844](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L844)

***

### StorageId \{#storageid\}

Ƭ **StorageId**: `string`

对存储中的某个文件的引用。

它会在 [StorageReader](../interfaces/server.StorageReader.md) 和 [StorageWriter](../interfaces/server.StorageWriter.md) 中使用，这两个接口可以分别通过 QueryCtx 和 MutationCtx 在 Convex 查询和变更函数中访问。

#### 定义于 \{#defined-in\}

[server/storage.ts:11](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L11)

***

### FileStorageId \{#filestorageid\}

Ƭ **FileStorageId**: [`GenericId`](values.md#genericid)&lt;`"_storage"`&gt; | [`StorageId`](server.md#storageid)

#### 定义于 \{#defined-in\}

[server/storage.ts:12](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L12)

***

### FileMetadata \{#filemetadata\}

Ƭ **FileMetadata**: `Object`

表示由 [storage.getMetadata](../interfaces/server.StorageReader.md#getmetadata) 返回的单个文件的元数据。

#### 类型声明 \{#type-declaration\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `storageId` | [`StorageId`](server.md#storageid) | 用于引用文件的 ID（例如通过 [storage.getUrl](../interfaces/server.StorageReader.md#geturl)） |
| `sha256` | `string` | 文件内容的 SHA-256 校验和（十六进制编码） |
| `size` | `number` | 文件的大小（字节数） |
| `contentType` | `string` | `null` | 文件的内容类型（Content-Type），如果在上传时有提供 |

#### 定义于 \{#defined-in\}

[server/storage.ts:18](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L18)

***

### SystemFields \{#systemfields\}

Ƭ **SystemFields**: `Object`

Convex 自动添加到文档中的字段，不包括 `_id`。

这是一个对象类型，用于将字段名映射到字段类型。

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 |
| :------ | :------ |
| `_creationTime` | `number` |

#### 定义于 \{#defined-in\}

[server/system&#95;fields.ts:11](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L11)

***

### IdField \{#idfield\}

Ƭ **IdField**&lt;`TableName`&gt;: `Object`

由 Convex 自动添加到文档中的 `_id` 字段。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `TableName` | extends `string` |

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 |
| :------ | :------ |
| `_id` | [`GenericId`](values.md#genericid)&lt;`TableName`&gt; |

#### 定义于 \{#defined-in\}

[server/system&#95;fields.ts:19](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L19)

***

### WithoutSystemFields \{#withoutsystemfields\}

Ƭ **WithoutSystemFields**&lt;`Document`&gt;: [`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;`Document`, keyof [`SystemFields`](server.md#systemfields) | `"_id"`&gt;&gt;

不包含 `_id` 和 `_creationTime` 等系统字段的 Convex 文档。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Document` | extends [`GenericDocument`](server.md#genericdocument) |

#### 定义在 \{#defined-in\}

[server/system&#95;fields.ts:28](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L28)

***

### WithOptionalSystemFields \{#withoptionalsystemfields\}

Ƭ **WithOptionalSystemFields**&lt;`Document`&gt;: [`Expand`](server.md#expand)&lt;[`WithoutSystemFields`](server.md#withoutsystemfields)&lt;`Document`&gt; &amp; `Partial`&lt;`Pick`&lt;`Document`, keyof [`SystemFields`](server.md#systemfields) | `"_id"`&gt;&gt;&gt;

表示一个 Convex 文档，其中 `_id`、`_creationTime` 等系统字段是可选的。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Document` | 继承自 [`GenericDocument`](server.md#genericdocument) |

#### 定义于 \{#defined-in\}

[server/system&#95;fields.ts:37](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L37)

***

### SystemIndexes \{#systemindexes\}

Ƭ **SystemIndexes**: `Object`

Convex 自动为每个表添加的索引。

这是一个对象，用于将索引名称映射到索引字段路径。

#### 类型声明 \{#type-declaration\}

| 名称 | 类型 |
| :------ | :------ |
| `by_id` | [`"_id"`] |
| `by_creation_time` | [`"_creationTime"`] |

#### 定义于 \{#defined-in\}

[server/system&#95;fields.ts:48](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L48)

***

### IndexTiebreakerField \{#indextiebreakerfield\}

Ƭ **IndexTiebreakerField**: `"_creationTime"`

如果所有其他字段都相同，Convex 会自动在每个索引的末尾追加 &quot;&#95;creationTime&quot;，用于打破并列。

#### 定义于 \{#defined-in\}

[server/system&#95;fields.ts:61](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L61)

***

### VectorSearch \{#vectorsearch\}

Ƭ **VectorSearch**&lt;`DataModel`, `TableName`, `IndexName`&gt;: (`tableName`: `TableName`, `indexName`: `IndexName`, `query`: [`VectorSearchQuery`](../interfaces/server.VectorSearchQuery.md)&lt;[`NamedTableInfo`](server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;, `IndexName`&gt;) =&gt; `Promise`&lt;&#123; `_id`: [`GenericId`](values.md#genericid)&lt;`TableName`&gt; ; `_score`: `number`  &#125;[]&gt;

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](server.md#genericdatamodel) |
| `TableName` | extends [`TableNamesInDataModel`](server.md#tablenamesindatamodel)&lt;`DataModel`&gt; |
| `IndexName` | extends [`VectorIndexNames`](server.md#vectorindexnames)&lt;[`NamedTableInfo`](server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt; |

#### 类型声明 \{#type-declaration\}

▸ (`tableName`, `indexName`, `query`): `Promise`&lt;&#123; `_id`: [`GenericId`](values.md#genericid)&lt;`TableName`&gt; ; `_score`: `number`  &#125;[]&gt;

##### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `tableName` | `TableName` |
| `indexName` | `IndexName` |
| `query` | [`VectorSearchQuery`](../interfaces/server.VectorSearchQuery.md)&lt;[`NamedTableInfo`](server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;, `IndexName`&gt; |

##### 返回值 \{#returns\}

`Promise`&lt;&#123; `_id`: [`GenericId`](values.md#genericid)&lt;`TableName`&gt; ; `_score`: `number`  &#125;[]&gt;

#### 定义于 \{#defined-in\}

[server/vector&#95;search.ts:55](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L55)

***

### Expand \{#expand\}

Ƭ **Expand**&lt;`ObjectType`&gt;: `ObjectType` extends `Record`&lt;`any`, `any`&gt; ? &#123; [Key in keyof ObjectType]: ObjectType[Key] &#125; : `never`

Hack！这个类型会让 TypeScript 以更简洁的方式显示对象类型。

从功能上讲，它对对象类型来说是恒等映射类型，但在实际使用中，它可以简化诸如 `A & B` 之类的表达式。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `ObjectType` | extends `Record`&lt;`any`, `any`&gt; |

#### 定义于 \{#defined-in\}

[type&#95;utils.ts:12](https://github.com/get-convex/convex-js/blob/main/src/type_utils.ts#L12)

***

### BetterOmit \{#betteromit\}

Ƭ **BetterOmit**&lt;`T`, `K`&gt;: &#123; [Property in keyof T as Property extends K ? never : Property]: T[Property] &#125;

一种 `Omit<>` 类型，它：

1. 适用于联合类型中的每个成员。
2. 保留底层类型的索引签名。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | `T` |
| `K` | extends keyof `T` |

#### 定义于 \{#defined-in\}

[type&#95;utils.ts:24](https://github.com/get-convex/convex-js/blob/main/src/type_utils.ts#L24)

## 变量 \{#variables\}

### anyApi \{#anyapi\}

• `Const` **anyApi**: [`AnyApi`](server.md#anyapi)

一个用于在不使用代码生成的项目中构造 [FunctionReference](server.md#functionreference) 的实用工具。

你可以这样创建对某个函数的引用：

```js
const reference = anyApi.myModule.myFunction;
```

这允许你访问任意路径，而不受你项目中目录和模块结构的限制。所有函数引用的类型都是
AnyFunctionReference。

如果你使用代码生成，请改为使用 `convex/_generated/api` 中的 `api`。这样类型安全性更高，并且能在你的编辑器中提供更好的自动补全体验。

#### 定义于 \{#defined-in\}

[server/api.ts:427](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L427)

***

### paginationOptsValidator \{#paginationoptsvalidator\}

• `Const` **paginationOptsValidator**: [`VObject`](../classes/values.VObject.md)&lt;&#123; `id`: `undefined` | `number` ; `endCursor`: `undefined` | `null` | `string` ; `maximumRowsRead`: `undefined` | `number` ; `maximumBytesRead`: `undefined` | `number` ; `numItems`: `number` ; `cursor`: `null` | `string`  &#125;, &#123; `numItems`: [`VFloat64`](../classes/values.VFloat64.md)&lt;`number`, `"required"`&gt; ; `cursor`: [`VUnion`](../classes/values.VUnion.md)&lt;`null` | `string`, [[`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"required"`, `never`&gt; ; `endCursor`: [`VUnion`](../classes/values.VUnion.md)&lt;`undefined` | `null` | `string`, [[`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"optional"`, `never`&gt; ; `id`: [`VFloat64`](../classes/values.VFloat64.md)&lt;`undefined` | `number`, `"optional"`&gt; ; `maximumRowsRead`: [`VFloat64`](../classes/values.VFloat64.md)&lt;`undefined` | `number`, `"optional"`&gt; ; `maximumBytesRead`: [`VFloat64`](../classes/values.VFloat64.md)&lt;`undefined` | `number`, `"optional"`&gt;  &#125;, `"required"`, `"id"` | `"numItems"` | `"cursor"` | `"endCursor"` | `"maximumRowsRead"` | `"maximumBytesRead"`&gt;

适用于 [PaginationOptions](../interfaces/server.PaginationOptions.md) 的[验证器](values.md#validator)。

它包含标准的 [PaginationOptions](../interfaces/server.PaginationOptions.md) 属性，以及一个可选的、用于“cache-busting”（避免缓存命中）的 `id` 属性，该属性会被 [usePaginatedQuery](react.md#usepaginatedquery) 使用。

#### 定义于 \{#defined-in\}

[server/pagination.ts:133](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L133)

***

### ROUTABLE_HTTP_METHODS \{#routable_http_methods\}

• `Const` **ROUTABLE&#95;HTTP&#95;METHODS**: readonly [`"GET"`, `"POST"`, `"PUT"`, `"DELETE"`, `"OPTIONS"`, `"PATCH"`]

Convex HTTP 操作所支持的 HTTP 方法列表。

HEAD 由 Convex 通过执行 GET 请求并去掉响应体来处理。
CONNECT 不受支持，且将来也不会被支持。
TRACE 不受支持，且将来也不会被支持。

#### 定义于 \{#defined-in\}

[server/router.ts:14](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L14)

## 函数 \{#functions\}

### getFunctionName \{#getfunctionname\}

▸ **getFunctionName**(`functionReference`): `string`

从 [FunctionReference](server.md#functionreference) 中获取函数的名称。

名称是一个类似 `"myDir/myModule:myFunction"` 的字符串。如果该函数的导出名称为 `"default"`，则会省略函数名（例如 `"myDir/myModule"`）。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `functionReference` | `AnyFunctionReference` | 用于获取其名称的 [FunctionReference](server.md#functionreference)。 |

#### 返回值 \{#returns\}

`string`

表示函数名的字符串。

#### 定义于 \{#defined-in\}

[server/api.ts:78](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L78)

***

### makeFunctionReference \{#makefunctionreference\}

▸ **makeFunctionReference**&lt;`type`, `args`, `ret`&gt;(`name`): [`FunctionReference`](server.md#functionreference)&lt;`type`, `"public"`, `args`, `ret`&gt;

`FunctionReference` 通常由代码生成产生，但在自定义客户端中，
有时手动构造一个这样的引用会很有用。

真实的函数引用在运行时是空对象，但对于不使用代码生成的测试和客户端，
可以使用普通对象来实现相同的接口。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `type` | 继承自 [`FunctionType`](server.md#functiontype) |
| `args` | 继承自 [`DefaultFunctionArgs`](server.md#defaultfunctionargs) = `any` |
| `ret` | `any` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `name` | `string` | 函数标识符。例如：`path/to/file:functionName` |

#### 返回 \{#returns\}

[`FunctionReference`](server.md#functionreference)&lt;`type`, `"public"`, `args`, `ret`&gt;

#### 定义于 \{#defined-in\}

[server/api.ts:122](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L122)

***

### filterApi \{#filterapi\}

▸ **filterApi**&lt;`API`, `Predicate`&gt;(`api`): [`FilterApi`](server.md#filterapi)&lt;`API`, `Predicate`&gt;

给定一个类型为 API 的 api 和一个 `FunctionReference` 子类型，返回一个仅包含匹配函数引用的 api 对象。

```ts
const q = filterApi<typeof api, FunctionReference<"query">>(api)
```

#### 类型参数 \{#type-parameters\}

| 名称 |
| :------ |
| `API` |
| `Predicate` |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `api` | `API` |

#### 返回 \{#returns\}

[`FilterApi`](server.md#filterapi)&lt;`API`, `Predicate`&gt;

#### 定义于 \{#defined-in\}

[server/api.ts:301](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L301)

***

### createFunctionHandle \{#createfunctionhandle\}

▸ **createFunctionHandle**&lt;`Type`, `Args`, `ReturnType`&gt;(`functionReference`): `Promise`&lt;[`FunctionHandle`](server.md#functionhandle)&lt;`Type`, `Args`, `ReturnType`&gt;&gt;

创建一个指向 Convex 函数的可序列化引用。
将该引用传递给另一个组件，可以让该组件在当前函数执行期间或任意之后的时间调用此函数。
函数句柄的用法类似于 `api.folder.function` 的函数引用（FunctionReferences），
例如 `ctx.scheduler.runAfter(0, functionReference, args)`。

函数引用在多次代码发布之间是稳定的，但它所指向的 Convex 函数有可能已经不存在。

这是组件的一项特性，目前处于 beta 阶段。
此 API 尚不稳定，后续版本中可能会发生变更。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Type` | extends [`FunctionType`](server.md#functiontype) |
| `Args` | extends [`DefaultFunctionArgs`](server.md#defaultfunctionargs) |
| `ReturnType` | `ReturnType` |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `functionReference` | [`FunctionReference`](server.md#functionreference)&lt;`Type`, `"public"` | `"internal"`, `Args`, `ReturnType`&gt; |

#### 返回值 \{#returns\}

`Promise`&lt;[`FunctionHandle`](server.md#functionhandle)&lt;`Type`, `Args`, `ReturnType`&gt;&gt;

#### 定义在 \{#defined-in\}

[server/components/index.ts:54](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L54)

***

### defineComponent \{#definecomponent\}

▸ **defineComponent**&lt;`Exports`&gt;(`name`): [`ComponentDefinition`](server.md#componentdefinition)&lt;`Exports`&gt;

定义一个组件，即 Convex 部署中的一部分，包含带命名空间的资源。

模块（例如 &quot;cool-component/convex.config.js&quot;）的默认导出
是一个 `@link ComponentDefinition&#125;，但在对组件定义进行求值时，
它的类型是本函数的类型签名。

@param name 名称必须由字母数字字符和下划线组成。通常使用
类似 `"onboarding_flow_tracker"` 这样的全小写加下划线形式。

这是组件的一项特性，目前处于 beta 阶段。
此 API 尚不稳定，后续版本中可能会发生变化。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Exports` | extends `ComponentExports` = `any` |

#### 参数 \{#parameters\}

| 参数名 | 类型 |
| :------ | :------ |
| `name` | `string` |

#### 返回值 \{#returns\}

[`ComponentDefinition`](server.md#componentdefinition)&lt;`Exports`&gt;

#### 定义于 \{#defined-in\}

[server/components/index.ts:371](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L371)

***

### defineApp \{#defineapp\}

▸ **defineApp**(): `AppDefinition`

将组件（Convex 部署中可复用的部分）接入到此 Convex 应用中。

这是组件的一项功能，目前处于测试阶段（beta）。
此 API 尚不稳定，后续版本中可能会发生变化。

#### 返回值 \{#returns\}

`AppDefinition`

#### 定义于 \{#defined-in\}

[server/components/index.ts:397](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L397)

***

### componentsGeneric \{#componentsgeneric\}

▸ **componentsGeneric**(): [`AnyChildComponents`](server.md#anychildcomponents)

#### 返回值 \{#returns\}

[`AnyChildComponents`](server.md#anychildcomponents)

#### 定义于 \{#defined-in\}

[server/components/index.ts:452](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L452)

***

### getFunctionAddress \{#getfunctionaddress\}

▸ **getFunctionAddress**(`functionReference`): &#123; `functionHandle`: `string` = functionReference; `name?`: `undefined` ; `reference?`: `undefined` = referencePath &#125; | &#123; `functionHandle?`: `undefined` = functionReference; `name`: `any` ; `reference?`: `undefined` = referencePath &#125; | &#123; `functionHandle?`: `undefined` = functionReference; `name?`: `undefined` ; `reference`: `string` = referencePath &#125;

#### 参数 \{#parameters\}

| 参数名 | 类型 |
| :------ | :------ |
| `functionReference` | `any` |

#### 返回值 \{#returns\}

&#123; `functionHandle`: `string` = functionReference; `name?`: `undefined` ; `reference?`: `undefined` = referencePath &#125; | &#123; `functionHandle?`: `undefined` = functionReference; `name`: `any` ; `reference?`: `undefined` = referencePath &#125; | &#123; `functionHandle?`: `undefined` = functionReference; `name?`: `undefined` ; `reference`: `string` = referencePath &#125;

#### 定义于 \{#defined-in\}

[server/components/paths.ts:20](https://github.com/get-convex/convex-js/blob/main/src/server/components/paths.ts#L20)

***

### cronJobs \{#cronjobs\}

▸ **cronJobs**(): [`Crons`](../classes/server.Crons.md)

创建一个 CronJobs 对象，用于计划执行定期任务。

```js
// convex/crons.js
import { cronJobs } from 'convex/server';
import { api } from "./_generated/api";

const crons = cronJobs();
crons.weekly(
  "weekly re-engagement email",
  {
    hourUTC: 17, // (太平洋时间上午 9:30/太平洋夏令时间上午 10:30)
    minuteUTC: 30,
  },
  api.emails.send
)
export default crons;
```

#### 返回值 \{#returns\}

[`Crons`](../classes/server.Crons.md)

#### 定义于 \{#defined-in\}

[server/cron.ts:180](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L180)

***

### mutationGeneric \{#mutationgeneric\}

▸ **mutationGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`mutation`): [`RegisteredMutation`](server.md#registeredmutation)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

在此 Convex 应用的公共 API 中定义一个变更。

此函数可以修改你的 Convex 数据库，并且可以从客户端调用。

如果你在使用代码生成，请使用 `convex/_generated/server.d.ts` 中的 `mutation` 函数，它已经根据你的数据模型进行了类型定义。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `mutation` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

#### 返回值 \{#returns\}

[`RegisteredMutation`](server.md#registeredmutation)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

封装后的变更函数。将其作为 `export` 导出，以便命名并在其他模块中使用。

#### 定义于 \{#defined-in\}

[server/registration.ts:608](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L608)

***

### internalMutationGeneric \{#internalmutationgeneric\}

▸ **internalMutationGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`mutation`): [`RegisteredMutation`](server.md#registeredmutation)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

定义一个仅能从其他 Convex 函数调用的变更（客户端不可访问）。

此函数被允许修改你的 Convex 数据库，但客户端无法调用它。

如果你使用代码生成，请在
`convex/_generated/server.d.ts` 中使用 `internalMutation` 函数，它已经根据你的数据模型进行了类型定义。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `mutation` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

#### 返回值 \{#returns\}

[`RegisteredMutation`](server.md#registeredmutation)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

包装后的变更。通过 `export` 导出它，以便为其命名并使其可访问。

#### 定义于 \{#defined-in\}

[server/registration.ts:608](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L608)

***

### queryGeneric \{#querygeneric\}

▸ **queryGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`query`): [`RegisteredQuery`](server.md#registeredquery)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

在此 Convex 应用的公共 API 中定义一个查询。

此函数将被允许读取你的 Convex 数据库，并且可以从客户端调用。

如果你使用代码生成，请在 `convex/_generated/server.d.ts` 中使用为你的数据模型提供类型定义的 `query` 函数。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `query` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

#### 返回值 \{#returns\}

[`RegisteredQuery`](server.md#registeredquery)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

被封装的查询。将其作为 `export` 导出，以便为其命名并使其可访问。

#### 定义于 \{#defined-in\}

[server/registration.ts:794](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L794)

***

### internalQueryGeneric \{#internalquerygeneric\}

▸ **internalQueryGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`query`): [`RegisteredQuery`](server.md#registeredquery)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

定义一个只能被其他 Convex 函数访问（客户端无法访问）的查询。

此函数可以从你的 Convex 数据库中读取数据，但客户端无法直接访问它。

如果你在使用代码生成，请在
`convex/_generated/server.d.ts` 中使用针对你的数据模型完成类型标注的 `internalQuery` 函数。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `query` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

#### 返回值 \{#returns\}

[`RegisteredQuery`](server.md#registeredquery)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

封装后的查询。将其作为 `export` 导出，这样可以为其命名并使其可访问。

#### 定义于 \{#defined-in\}

[server/registration.ts:794](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L794)

***

### actionGeneric \{#actiongeneric\}

▸ **actionGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`func`): [`RegisteredAction`](server.md#registeredaction)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

在此 Convex 应用的公共 API 中定义一个操作。

如果你在使用代码生成，请使用 `convex/_generated/server.d.ts` 中的 `action` 函数，它已经根据你的数据模型进行了类型标注。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `func` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` | 要注册的函数。它的第一个参数是一个 [GenericActionCtx](../interfaces/server.GenericActionCtx.md)。 |

#### 返回值 \{#returns\}

[`RegisteredAction`](server.md#registeredaction)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

包装后的函数。将其作为一个 `export` 导出，以便为其命名并使其可被访问。

#### 定义于 \{#defined-in\}

[server/registration.ts:972](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L972)

***

### internalActionGeneric \{#internalactiongeneric\}

▸ **internalActionGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`func`): [`RegisteredAction`](server.md#registeredaction)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

定义一个只能从其他 Convex 函数中调用的操作（客户端无法访问）。

如果你在使用代码生成，请在
`convex/_generated/server.d.ts` 中使用 `internalAction` 函数，它已经根据你的数据模型完成了类型标注。

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `func` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` | 该函数。它会接收一个 [GenericActionCtx](../interfaces/server.GenericActionCtx.md) 作为第一个参数。 |

#### 返回 \{#returns\}

[`RegisteredAction`](server.md#registeredaction)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

包装后的函数。将其作为 `export` 导出，从而为其命名并使其可访问。

#### 定义于 \{#defined-in\}

[server/registration.ts:972](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L972)

***

### httpActionGeneric \{#httpactiongeneric\}

▸ **httpActionGeneric**(`func`): [`PublicHttpAction`](server.md#publichttpaction)

定义一个 Convex HTTP 操作。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `func` | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;[`GenericDataModel`](server.md#genericdatamodel)&gt;, `request`: `Request`) =&gt; `Promise`&lt;`Response`&gt; | 该函数。它的第一个参数是 [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)，第二个参数是 `Request` 对象。 |

#### 返回值 \{#returns\}

[`PublicHttpAction`](server.md#publichttpaction)

封装后的函数。在 `convex/http.js` 中将某个 URL 路径路由到该函数。

#### 定义在 \{#defined-in\}

[server/impl/registration&#95;impl.ts:467](https://github.com/get-convex/convex-js/blob/main/src/server/impl/registration_impl.ts#L467)

***

### paginationResultValidator \{#paginationresultvalidator\}

▸ **paginationResultValidator**&lt;`T`&gt;(`itemValidator`): [`VObject`](../classes/values.VObject.md)&lt;&#123; `splitCursor`: `undefined` | `null` | `string` ; `pageStatus`: `undefined` | `null` | `"SplitRecommended"` | `"SplitRequired"` ; `page`: `T`[`"type"`][] ; `continueCursor`: `string` ; `isDone`: `boolean`  &#125;, &#123; `page`: [`VArray`](../classes/values.VArray.md)&lt;`T`[`"type"`][], `T`, `"required"`&gt; ; `continueCursor`: [`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt; ; `isDone`: [`VBoolean`](../classes/values.VBoolean.md)&lt;`boolean`, `"required"`&gt; ; `splitCursor`: [`VUnion`](../classes/values.VUnion.md)&lt;`undefined` | `null` | `string`, [[`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"optional"`, `never`&gt; ; `pageStatus`: [`VUnion`](../classes/values.VUnion.md)&lt;`undefined` | `null` | `"SplitRecommended"` | `"SplitRequired"`, [[`VLiteral`](../classes/values.VLiteral.md)&lt;`"SplitRecommended"`, `"required"`&gt;, [`VLiteral`](../classes/values.VLiteral.md)&lt;`"SplitRequired"`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"optional"`, `never`&gt;  &#125;, `"required"`, `"page"` | `"continueCursor"` | `"isDone"` | `"splitCursor"` | `"pageStatus"`&gt;

用于 [PaginationResult](../interfaces/server.PaginationResult.md) 的 [Validator](values.md#validator) 工厂函数。

使用给定的条目验证器，为调用 [paginate](../interfaces/server.OrderedQuery.md#paginate) 所返回的结果创建一个验证器。

例如：

```ts
const paginationResultValidator = paginationResultValidator(v.object({
  _id: v.id("users"),
  _creationTime: v.number(),
  name: v.string(),
}));
```

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `T` | extends [`Validator`](values.md#validator)&lt;[`Value`](values.md#value), `"required"`, `string`&gt; |

#### 参数 \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `itemValidator` | `T` | 页面中每个项的验证器 |

#### 返回 \{#returns\}

[`VObject`](../classes/values.VObject.md)&lt;&#123; `splitCursor`: `undefined` | `null` | `string` ; `pageStatus`: `undefined` | `null` | `"SplitRecommended"` | `"SplitRequired"` ; `page`: `T`[`"type"`][] ; `continueCursor`: `string` ; `isDone`: `boolean`  &#125;, &#123; `page`: [`VArray`](../classes/values.VArray.md)&lt;`T`[`"type"`][], `T`, `"required"`&gt; ; `continueCursor`: [`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt; ; `isDone`: [`VBoolean`](../classes/values.VBoolean.md)&lt;`boolean`, `"required"`&gt; ; `splitCursor`: [`VUnion`](../classes/values.VUnion.md)&lt;`undefined` | `null` | `string`, [[`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"optional"`, `never`&gt; ; `pageStatus`: [`VUnion`](../classes/values.VUnion.md)&lt;`undefined` | `null` | `"SplitRecommended"` | `"SplitRequired"`, [[`VLiteral`](../classes/values.VLiteral.md)&lt;`"SplitRecommended"`, `"required"`&gt;, [`VLiteral`](../classes/values.VLiteral.md)&lt;`"SplitRequired"`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"optional"`, `never`&gt;  &#125;, `"required"`, `"page"` | `"continueCursor"` | `"isDone"` | `"splitCursor"` | `"pageStatus"`&gt;

用于校验分页结果的验证器

#### 定义于 \{#defined-in\}

[server/pagination.ts:162](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L162)

***

### httpRouter \{#httprouter\}

▸ **httpRouter**(): [`HttpRouter`](../classes/server.HttpRouter.md)

返回一个新的 [HttpRouter](../classes/server.HttpRouter.md) 对象。

#### 返回 \{#returns\}

[`HttpRouter`](../classes/server.HttpRouter.md)

#### 定义于 \{#defined-in\}

[server/router.ts:47](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L47)

***

### defineTable \{#definetable\}

▸ **defineTable**&lt;`DocumentSchema`&gt;(`documentSchema`): [`TableDefinition`](../classes/server.TableDefinition.md)&lt;`DocumentSchema`&gt;

在模式中定义一个表。

你可以把文档的模式指定为一个对象，例如

```ts
defineTable({
  field: v.string()
});
```

或者作为模式类型，例如：

```ts
defineTable(
 v.union(
   v.object({...}),
   v.object({...})
 )
);
```

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DocumentSchema` | extends [`Validator`](values.md#validator)&lt;`Record`&lt;`string`, `any`&gt;, `"required"`, `any`&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `documentSchema` | `DocumentSchema` | 存储在此表中的文档的类型。 |

#### 返回值 \{#returns\}

[`TableDefinition`](../classes/server.TableDefinition.md)&lt;`DocumentSchema`&gt;

该表的 [`TableDefinition`](../classes/server.TableDefinition.md)。

#### 定义位置 \{#defined-in\}

[server/schema.ts:593](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L593)

▸ **defineTable**&lt;`DocumentSchema`&gt;(`documentSchema`): [`TableDefinition`](../classes/server.TableDefinition.md)&lt;[`VObject`](../classes/values.VObject.md)&lt;[`ObjectType`](values.md#objecttype)&lt;`DocumentSchema`&gt;, `DocumentSchema`&gt;&gt;

在模式中定义一个表。

你可以将文档的模式指定为一个对象，例如

```ts
defineTable({
  field: v.string()
});
```

或作为一种模式类型，例如

```ts
defineTable(
 v.union(
   v.object({...}),
   v.object({...})
 )
);
```

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `DocumentSchema` | extends `Record`&lt;`string`, [`GenericValidator`](values.md#genericvalidator)&gt; |

#### 参数 \{#parameters\}

| 名称 | 类型 | 说明 |
| :------ | :------ | :------ |
| `documentSchema` | `DocumentSchema` | 存储在此表中的文档类型。 |

#### 返回值 \{#returns\}

[`TableDefinition`](../classes/server.TableDefinition.md)&lt;[`VObject`](../classes/values.VObject.md)&lt;[`ObjectType`](values.md#objecttype)&lt;`DocumentSchema`&gt;, `DocumentSchema`&gt;&gt;

此表的 [TableDefinition](../classes/server.TableDefinition.md)。

#### 定义于 \{#defined-in\}

[server/schema.ts:621](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L621)

***

### defineSchema \{#defineschema\}

▸ **defineSchema**&lt;`Schema`, `StrictTableNameTypes`&gt;(`schema`, `options?`): [`SchemaDefinition`](../classes/server.SchemaDefinition.md)&lt;`Schema`, `StrictTableNameTypes`&gt;

为该 Convex 项目定义模式。

应当从 `convex/` 目录下的 `schema.ts` 文件中导出，例如：

```ts
export default defineSchema({
  ...
});
```

#### 类型参数 \{#type-parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `Schema` | 扩展自 [`GenericSchema`](server.md#genericschema) |
| `StrictTableNameTypes` | 扩展自 `boolean`，默认为 `true` |

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `schema` | `Schema` | 一份从表名到本项目中所有表的 [TableDefinition](../classes/server.TableDefinition.md) 的映射。 |
| `options?` | [`DefineSchemaOptions`](../interfaces/server.DefineSchemaOptions.md)&lt;`StrictTableNameTypes`&gt; | 可选配置。完整说明请参阅 [DefineSchemaOptions](../interfaces/server.DefineSchemaOptions.md)。 |

#### 返回值 \{#returns\}

[`SchemaDefinition`](../classes/server.SchemaDefinition.md)&lt;`Schema`, `StrictTableNameTypes`&gt;

模式。

#### 定义于 \{#defined-in\}

[server/schema.ts:769](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L769)