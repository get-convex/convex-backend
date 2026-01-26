---
title: "server.js"
sidebar_position: 4
description:
  "用于实现 Convex 查询、变更和操作的自动生成工具"
---

<Admonition type="caution" title="此代码是自动生成的">
  这些导出内容在 `convex` 包中无法直接使用！

  你必须运行 `npx convex dev` 来生成 `convex/_generated/server.js`
  和 `convex/_generated/server.d.ts`。
</Admonition>

用于实现服务端 Convex 查询和变更函数的自动生成工具。

## 函数 \{#functions\}

### query \{#query\}

▸ **query**(`func`): [`RegisteredQuery`](/api/modules/server#registeredquery)

在此 Convex 应用的公共 API 中定义一个查询。

该函数可以读取你的 Convex 数据库，并可从客户端调用。

这是针对你的应用数据模型做了类型限定的 [`queryGeneric`](/api/modules/server#querygeneric) 的别名。

#### 参数 \{#parameters\}

| 名称   | 描述                                                                                     |
| :----- | :---------------------------------------------------------------------------------------- |
| `func` | 查询函数。它接收一个 [QueryCtx](server.md#queryctx) 作为其第一个参数。 |

#### 返回值 \{#returns\}

[`RegisteredQuery`](/api/modules/server#registeredquery)

封装后的查询。将其作为一个 `export` 包含在内，以便为其命名并使其可被访问。

***

### internalQuery \{#internalquery\}

▸ **internalQuery**(`func`):
[`RegisteredQuery`](/api/modules/server#registeredquery)

定义一个仅能从其他 Convex 函数调用的查询（客户端无法调用）。

此函数可以从你的 Convex 数据库中读取数据，但客户端无法访问它。

这是
[`internalQueryGeneric`](/api/modules/server#internalquerygeneric)
的一个别名，并且具有针对你的应用数据模型的类型定义。

#### 参数 \{#parameters\}

| Name   | Description                                                                                 |
| :----- | :------------------------------------------------------------------------------------------ |
| `func` | 查询函数。它会接收 [QueryCtx](server.md#queryctx) 作为第一个参数。 |

#### 返回值 \{#returns\}

[`RegisteredQuery`](/api/modules/server#registeredquery)

被包装的查询。将其作为一个 `export` 导出，以便为其命名并使其可被访问。

***

### mutation \{#mutation\}

▸ **mutation**(`func`):
[`RegisteredMutation`](/api/modules/server#registeredmutation)

在这个 Convex 应用的公共 API 中定义一个变更函数。

此函数被允许修改你的 Convex 数据库，并且可以从客户端调用。

这是 [`mutationGeneric`](/api/modules/server#mutationgeneric) 的一个别名，并且已经根据你的应用数据模型进行了类型标注。

#### 参数 \{#parameters\}

| Name   | Description                                                                                  |
| :----- | :------------------------------------------------------------------------------------------- |
| `func` | 变更函数。它接收 [MutationCtx](#mutationctx) 作为第一个参数。 |

#### 返回值 \{#returns\}

[`RegisteredMutation`](/api/modules/server#registeredmutation)

被包装的变更。将其作为 `export` 导出，以便为其命名并使其可被访问。

***

### internalMutation \{#internalmutation\}

▸ **internalMutation**(`func`):
[`RegisteredMutation`](/api/modules/server#registeredmutation)

定义一个只能从其他 Convex 函数调用的变更（客户端无法访问）。

此函数可以读取和写入你的 Convex 数据库，但不会对客户端开放。

这是
[`internalMutationGeneric`](/api/modules/server#internalmutationgeneric)
的一个别名，并且针对你的应用数据模型进行了类型定义。

#### 参数 \{#parameters\}

| 名称   | 描述                                                                                                  |
| :----- | :--------------------------------------------------------------------------------------------------- |
| `func` | 变更函数。它接收一个 [MutationCtx](server.md#mutationctx) 作为第一个参数。 |

#### 返回值 \{#returns\}

[`RegisteredMutation`](/api/modules/server#registeredmutation)

被包装的变更。将其作为一个 `export` 导出，以便为其命名并使其可被访问。

***

### action \{#action\}

▸ **action**(`func`): [`RegisteredAction`](/api/modules/server#registeredaction)

在此 Convex 应用的公共 API 中定义一个操作。

操作是一个函数，可以执行任意 JavaScript 代码，包括非确定性代码以及具有副作用的代码，比如调用第三方服务。它们可以在 Convex 的 JavaScript 运行环境中运行，或者通过 `"use node"` 指令在 Node.js 中运行。它们可以通过 [`ActionCtx`](#actionctx) 调用查询和变更函数，间接与数据库交互。

这是一个 [`actionGeneric`](/api/modules/server#actiongeneric) 的别名，并且已经根据你的应用数据模型进行了类型标注。

#### 参数 \{#parameters\}

| 名称   | 说明                                                                 |
| :----- | :-------------------------------------------------------------------- |
| `func` | 操作函数。它接收一个 [ActionCtx](#actionctx) 作为其第一个参数。 |

#### 返回值 \{#returns\}

[`RegisteredAction`](/api/modules/server#registeredaction)

被包装后的函数。将其作为 `export` 导出，以便为其命名并使其可被访问。

***

### internalAction \{#internalaction\}

▸ **internalAction**(`func`):
[`RegisteredAction`](/api/modules/server#registeredaction)

定义一个只能被其他 Convex 函数调用（而不能从客户端调用）的操作。

这是
[`internalActionGeneric`](/api/modules/server#internalactiongeneric)
的别名，并且针对你的应用数据模型进行了类型标注。

#### 参数 \{#parameters\}

| 名称   | 描述                                                                                         |
| :----- | :------------------------------------------------------------------------------------------ |
| `func` | 操作函数。它接收一个 [ActionCtx](server.md#actionctx) 作为第一个参数。 |

#### 返回值 \{#returns\}

[`RegisteredAction`](/api/modules/server#registeredaction)

封装后的操作。将其作为 `export` 导出，以便为其命名并使其可被访问。

***

### httpAction \{#httpaction\}

▸
**httpAction**(`func: (ctx: ActionCtx, request: Request) => Promise<Response>`):
[`PublicHttpAction`](/api/modules/server#publichttpaction)

#### 参数 \{#parameters\}

| 名称   | 类型                                                      | 描述                                                                                                                                                                              |
| :----- | :-------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `func` | `(ctx: ActionCtx, request: Request) => Promise<Response>` | 此函数的第一个参数是 [`ActionCtx`](/api/modules/server#actionctx)，第二个参数是 [`Request`](https://developer.mozilla.org/en-US/docs/Web/API/Request)。 |

#### 返回值 \{#returns\}

[`PublicHttpAction`](/api/modules/server#publichttpaction)

被包装后的函数。请从 `convex/http.js` 中导入此函数并将其接入路由即可使用。

## 类型 \{#types\}

### QueryCtx \{#queryctx\}

Ƭ **QueryCtx**: `Object`

一组可在 Convex 查询函数中使用的服务。

查询上下文会作为第一个参数传递给在服务器上运行的任何 Convex 查询函数。

这与 [MutationCtx](#mutationctx) 不同，因为其中所有服务都是只读的。

这是 [`GenericQueryCtx`](/api/interfaces/server.GenericQueryCtx) 的一个别名，其类型已根据你的应用的数据模型进行了限定。

#### 类型声明 \{#type-declaration\}

| 名称      | 类型                                                       |
| :-------- | :--------------------------------------------------------- |
| `db`      | [`DatabaseReader`](#databasereader)                        |
| `auth`    | [`Auth`](/api/interfaces/server.Auth.md)                   |
| `storage` | [`StorageReader`](/api/interfaces/server.StorageReader.md) |

***

### MutationCtx \{#mutationctx\}

Ƭ **MutationCtx**: `Object`

一组可在 Convex 变更函数中使用的服务。

变更上下文会作为第一个参数传给在服务器上运行的任何 Convex 变更函数。

它是[`GenericMutationCtx`](/api/interfaces/server.GenericMutationCtx)的一个别名，
并且根据你的应用数据模型进行了类型化。

#### 类型声明 \{#type-declaration\}

| 名称        | 类型                                                       |
| :---------- | :--------------------------------------------------------- |
| `db`        | [`DatabaseWriter`](#databasewriter)                        |
| `auth`      | [`Auth`](/api/interfaces/server.Auth.md)                   |
| `storage`   | [`StorageWriter`](/api/interfaces/server.StorageWriter.md) |
| `scheduler` | [`Scheduler`](/api/interfaces/server.Scheduler.md)         |

***

### ActionCtx \{#actionctx\}

Ƭ **ActionCtx**: `Object`

在 Convex 操作函数中可使用的一组服务。

操作上下文会作为第一个参数传递给在服务器上运行的任意 Convex 操作函数。

这是 [`ActionCtx`](/api/modules/server#actionctx) 的一个别名，并且已经根据你的应用数据模型进行了类型标注。

#### 类型声明 \{#type-declaration\}

| 名称           | 类型                                                                                                                                                                         |
| :------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `runQuery`     | (`name`: `string`, `args`?: `Record<string, Value>`) =&gt; `Promise<Value>`                                                                                                     |
| `runMutation`  | (`name`: `string`, `args`?: `Record<string, Value>`) =&gt; `Promise<Value>`                                                                                                     |
| `runAction`    | (`name`: `string`, `args`?: `Record<string, Value>`) =&gt; `Promise<Value>`                                                                                                     |
| `auth`         | [`Auth`](/api/interfaces/server.Auth.md)                                                                                                                                     |
| `scheduler`    | [`Scheduler`](/api/interfaces/server.Scheduler.md)                                                                                                                           |
| `storage`      | [`StorageActionWriter`](/api/interfaces/server.StorageActionWriter.md)                                                                                                       |
| `vectorSearch` | (`tableName`: `string`, `indexName`: `string`, `query`: [`VectorSearchQuery`](/api/interfaces/server.VectorSearchQuery.md)) =&gt; `Promise<Array<{ _id: Id, _score: number }>>` |

***

### DatabaseReader \{#databasereader\}

用于在 Convex 查询函数中从数据库读取数据的接口。

这是
[`GenericDatabaseReader`](/api/interfaces/server.GenericDatabaseReader)
的别名，并针对你的应用数据模型进行了类型限定。

### DatabaseWriter \{#databasewriter\}

一个在 Convex 的变更函数中用于读取和写入数据库的接口。

这是 [`GenericDatabaseWriter`](/api/interfaces/server.GenericDatabaseWriter) 的别名，
并针对你的应用数据模型进行了类型限定。