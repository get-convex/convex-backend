/* eslint-disable */
/**
 * Generated `ComponentApi` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type { FunctionReference } from "convex/server";

/**
 * A utility for referencing a Convex component's exposed API.
 *
 * Useful when expecting a parameter like `components.myComponent`.
 * Usage:
 * ```ts
 * async function myFunction(ctx: QueryCtx, component: ComponentApi) {
 *   return ctx.runQuery(component.someFile.someQuery, { ...args });
 * }
 * ```
 */
export type ComponentApi<Name extends string | undefined = string | undefined> =
  {
    cleanUp: {
      default: FunctionReference<"mutation", "internal", any, any, Name>;
    };
    errors: {
      throwConvexError: FunctionReference<"query", "internal", any, any, Name>;
      throwError: FunctionReference<"query", "internal", any, any, Name>;
    };
    fileStorage: {
      deleteById: FunctionReference<
        "mutation",
        "internal",
        { storageId: string },
        any,
        Name
      >;
      generateUploadUrl: FunctionReference<
        "mutation",
        "internal",
        any,
        any,
        Name
      >;
      get: FunctionReference<"query", "internal", { id: string }, any, Name>;
      getFile: FunctionReference<
        "action",
        "internal",
        { storageId: string },
        any,
        Name
      >;
      getUrl: FunctionReference<
        "query",
        "internal",
        { storageId: string },
        any,
        Name
      >;
      list: FunctionReference<"query", "internal", any, any, Name>;
      storeFile: FunctionReference<
        "action",
        "internal",
        { data: string },
        any,
        Name
      >;
    };
    functionHandles: {
      fromAction: FunctionReference<"action", "internal", any, any, Name>;
      fromQuery: FunctionReference<"query", "internal", any, any, Name>;
      getInternalHandle: FunctionReference<
        "query",
        "internal",
        { functionType: "query" | "mutation" | "action" },
        string,
        Name
      >;
    };
    scheduler: {
      listAllMessages: FunctionReference<"query", "internal", any, any, Name>;
      scheduleWithinComponent: FunctionReference<
        "mutation",
        "internal",
        { message: string },
        string,
        Name
      >;
      sendMessage: FunctionReference<
        "mutation",
        "internal",
        { message: string },
        any,
        Name
      >;
      status: FunctionReference<"query", "internal", { id: string }, any, Name>;
    };
    transact: {
      allMessages: FunctionReference<
        "query",
        "internal",
        {},
        Array<string>,
        Name
      >;
      sendButFail: FunctionReference<
        "mutation",
        "internal",
        { message: string },
        any,
        Name
      >;
      sendMessage: FunctionReference<
        "mutation",
        "internal",
        { message: string },
        any,
        Name
      >;
    };
  };
