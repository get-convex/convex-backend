---
title: "server.js"
sidebar_position: 4
description:
  "Generated utilities for implementing Convex queries, mutations, and actions"
---

<Admonition type="caution" title="This code is generated">

These exports are not directly available in the `convex` package!

Instead you must run `npx convex dev` to create `convex/_generated/server.js`
and `convex/_generated/server.d.ts`.

</Admonition>

Generated utilities for implementing server-side Convex query and mutation
functions.

## Functions

### query

▸ **query**(`func`): [`RegisteredQuery`](/api/modules/server#registeredquery)

Define a query in this Convex app's public API.

This function will be allowed to read your Convex database and will be
accessible from the client.

This is an alias of [`queryGeneric`](/api/modules/server#querygeneric) that is
typed for your app's data model.

#### Parameters

| Name   | Description                                                                             |
| :----- | :-------------------------------------------------------------------------------------- |
| `func` | The query function. It receives a [QueryCtx](server.md#queryctx) as its first argument. |

#### Returns

[`RegisteredQuery`](/api/modules/server#registeredquery)

The wrapped query. Include this as an `export` to name it and make it
accessible.

---

### internalQuery

▸ **internalQuery**(`func`):
[`RegisteredQuery`](/api/modules/server#registeredquery)

Define a query that is only accessible from other Convex functions (but not from
the client).

This function will be allowed to read from your Convex database. It will not be
accessible from the client.

This is an alias of
[`internalQueryGeneric`](/api/modules/server#internalquerygeneric) that is typed
for your app's data model.

#### Parameters

| Name   | Description                                                                             |
| :----- | :-------------------------------------------------------------------------------------- |
| `func` | The query function. It receives a [QueryCtx](server.md#queryctx) as its first argument. |

#### Returns

[`RegisteredQuery`](/api/modules/server#registeredquery)

The wrapped query. Include this as an `export` to name it and make it
accessible.

---

### mutation

▸ **mutation**(`func`):
[`RegisteredMutation`](/api/modules/server#registeredmutation)

Define a mutation in this Convex app's public API.

This function will be allowed to modify your Convex database and will be
accessible from the client.

This is an alias of [`mutationGeneric`](/api/modules/server#mutationgeneric)
that is typed for your app's data model.

#### Parameters

| Name   | Description                                                                             |
| :----- | :-------------------------------------------------------------------------------------- |
| `func` | The mutation function. It receives a [MutationCtx](#mutationctx) as its first argument. |

#### Returns

[`RegisteredMutation`](/api/modules/server#registeredmutation)

The wrapped mutation. Include this as an `export` to name it and make it
accessible.

---

### internalMutation

▸ **internalMutation**(`func`):
[`RegisteredMutation`](/api/modules/server#registeredmutation)

Define a mutation that is only accessible from other Convex functions (but not
from the client).

This function will be allowed to read and write from your Convex database. It
will not be accessible from the client.

This is an alias of
[`internalMutationGeneric`](/api/modules/server#internalmutationgeneric) that is
typed for your app's data model.

#### Parameters

| Name   | Description                                                                                      |
| :----- | :----------------------------------------------------------------------------------------------- |
| `func` | The mutation function. It receives a [MutationCtx](server.md#mutationctx) as its first argument. |

#### Returns

[`RegisteredMutation`](/api/modules/server#registeredmutation)

The wrapped mutation. Include this as an `export` to name it and make it
accessible.

---

### action

▸ **action**(`func`): [`RegisteredAction`](/api/modules/server#registeredaction)

Define an action in this Convex app's public API.

An action is a function which can execute any JavaScript code, including
non-deterministic code and code with side-effects, like calling third-party
services. They can be run in Convex's JavaScript environment or in Node.js using
the `"use node"` directive. They can interact with the database indirectly by
calling queries and mutations using the [`ActionCtx`](#actionctx).

This is an alias of [`actionGeneric`](/api/modules/server#actiongeneric) that is
typed for your app's data model.

#### Parameters

| Name   | Description                                                                        |
| :----- | :--------------------------------------------------------------------------------- |
| `func` | The action function. It receives an [ActionCtx](#actionctx) as its first argument. |

#### Returns

[`RegisteredAction`](/api/modules/server#registeredaction)

The wrapped function. Include this as an `export` to name it and make it
accessible.

---

### internalAction

▸ **internalAction**(`func`):
[`RegisteredAction`](/api/modules/server#registeredaction)

Define an action that is only accessible from other Convex functions (but not
from the client).

This is an alias of
[`internalActionGeneric`](/api/modules/server#internalactiongeneric) that is
typed for your app's data model.

#### Parameters

| Name   | Description                                                                                 |
| :----- | :------------------------------------------------------------------------------------------ |
| `func` | The action function. It receives an [ActionCtx](server.md#actionctx) as its first argument. |

#### Returns

[`RegisteredAction`](/api/modules/server#registeredaction)

The wrapped action. Include this as an `export` to name it and make it
accessible.

---

### httpAction

▸
**httpAction**(`func: (ctx: ActionCtx, request: Request) => Promise<Response>`):
[`PublicHttpAction`](/api/modules/server#publichttpaction)

#### Parameters

| Name   | Type                                                      | Description                                                                                                                                                                                         |
| :----- | :-------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `func` | `(ctx: ActionCtx, request: Request) => Promise<Response>` | The function. It receives an [`ActionCtx`](/api/modules/server#actionctx) as its first argument and a [`Request`](https://developer.mozilla.org/en-US/docs/Web/API/Request) as its second argument. |

#### Returns

[`PublicHttpAction`](/api/modules/server#publichttpaction)

The wrapped function. Import this function from `convex/http.js` and route it to
hook it up.

## Types

### QueryCtx

Ƭ **QueryCtx**: `Object`

A set of services for use within Convex query functions.

The query context is passed as the first argument to any Convex query function
run on the server.

This differs from the [MutationCtx](#mutationctx) because all of the services
are read-only.

This is an alias of [`GenericQueryCtx`](/api/interfaces/server.GenericQueryCtx)
that is typed for your app's data model.

#### Type declaration

| Name               | Type                                                       |
| :----------------- | :--------------------------------------------------------- |
| `db`               | [`DatabaseReader`](#databasereader)                        |
| `auth`             | [`Auth`](/api/interfaces/server.Auth.md)                   |
| `storage`          | [`StorageReader`](/api/interfaces/server.StorageReader.md) |
| `setLogAttributes` | [`setLogAttributes`](#setlogattributes)                    |

---

### MutationCtx

Ƭ **MutationCtx**: `Object`

A set of services for use within Convex mutation functions.

The mutation context is passed as the first argument to any Convex mutation
function run on the server.

This is an alias of
[`GenericMutationCtx`](/api/interfaces/server.GenericMutationCtx) that is typed
for your app's data model.

#### Type declaration

| Name               | Type                                                       |
| :----------------- | :--------------------------------------------------------- |
| `db`               | [`DatabaseWriter`](#databasewriter)                        |
| `auth`             | [`Auth`](/api/interfaces/server.Auth.md)                   |
| `storage`          | [`StorageWriter`](/api/interfaces/server.StorageWriter.md) |
| `scheduler`        | [`Scheduler`](/api/interfaces/server.Scheduler.md)         |
| `setLogAttributes` | [`setLogAttributes`](#setlogattributes)                    |

---

### ActionCtx

Ƭ **ActionCtx**: `Object`

A set of services for use within Convex action functions.

The action context is passed as the first argument to any Convex action function
run on the server.

This is an alias of [`ActionCtx`](/api/modules/server#actionctx) that is typed
for your app's data model.

#### Type declaration

| Name               | Type                                                                                                                                                                         |
| :----------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `runQuery`         | (`name`: `string`, `args`?: `Record<string, Value>`) => `Promise<Value>`                                                                                                     |
| `runMutation`      | (`name`: `string`, `args`?: `Record<string, Value>`) => `Promise<Value>`                                                                                                     |
| `runAction`        | (`name`: `string`, `args`?: `Record<string, Value>`) => `Promise<Value>`                                                                                                     |
| `auth`             | [`Auth`](/api/interfaces/server.Auth.md)                                                                                                                                     |
| `scheduler`        | [`Scheduler`](/api/interfaces/server.Scheduler.md)                                                                                                                           |
| `storage`          | [`StorageActionWriter`](/api/interfaces/server.StorageActionWriter.md)                                                                                                       |
| `vectorSearch`     | (`tableName`: `string`, `indexName`: `string`, `query`: [`VectorSearchQuery`](/api/interfaces/server.VectorSearchQuery.md)) => `Promise<Array<{ _id: Id, _score: number }>>` |
| `setLogAttributes` | [`setLogAttributes`](#setlogattributes)                                                                                                                                      |

---

### setLogAttributes

▸ **setLogAttributes**(attrs: `Record<string, string | number | boolean>`):
`void`

Set custom attributes that will be included in the
[`function_execution`](/production/integrations/log-streams/log-streams#function_execution-events)
log event. Useful for adding context like user IDs, operation types, or request
metadata for observability and debugging.

Available on all function context types: [`QueryCtx`](#queryctx),
[`MutationCtx`](#mutationctx), and [`ActionCtx`](#actionctx).

#### Example

```typescript
export const sendMessage = mutation({
  args: { body: v.string(), channelId: v.id("channels") },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    ctx.setLogAttributes({
      user_id: identity?.subject ?? "anonymous",
      channel_id: args.channelId,
      operation: "send_message",
    });
    // ... rest of function
  },
});
```

#### Parameters

| Name    | Type                                          | Description                                  |
| :------ | :-------------------------------------------- | :------------------------------------------- |
| `attrs` | `Record<string, string \| number \| boolean>` | Key-value pairs to include in the log event. |

#### Limits

- **Maximum 10 keys** per function execution
- **Maximum 1KB total size** for all keys and values combined
- **Keys** must contain only alphanumeric characters, underscores, and dots
  (`a-z`, `A-Z`, `0-9`, `_`, `.`)
- **Values** must be strings, numbers, or booleans

If limits are exceeded, attributes will be partially applied and a warning will
be logged. The function will continue executing normally.

#### Behavior

- Can be called multiple times; attributes are merged with last-write-wins for
  duplicate keys
- Attributes appear in the `custom_attributes` field of `function_execution` log
  events
- Empty attribute objects are omitted from log output

---

### DatabaseReader

An interface to read from the database within Convex query functions.

This is an alias of
[`GenericDatabaseReader`](/api/interfaces/server.GenericDatabaseReader) that is
typed for your app's data model.

---

### DatabaseWriter

An interface to read from and write to the database within Convex mutation
functions.

This is an alias of
[`GenericDatabaseWriter`](/api/interfaces/server.GenericDatabaseWriter) that is
typed for your app's data model.
