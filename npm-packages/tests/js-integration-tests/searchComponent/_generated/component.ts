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
    foods: {
      insertRow: FunctionReference<
        "mutation",
        "internal",
        {
          bOrC: string;
          cuisine: string;
          description: string;
          embedding: Array<number>;
          theLetterA: string;
        },
        any,
        Name
      >;
      populate: FunctionReference<"action", "internal", {}, any, Name>;
      queryDocs: FunctionReference<
        "query",
        "internal",
        { ids: Array<string> },
        any,
        Name
      >;
    };
    textSearch: {
      fullTextSearchMutation: FunctionReference<
        "mutation",
        "internal",
        { cuisine?: string; query: string },
        any,
        Name
      >;
      fullTextSearchMutationWithWrite: FunctionReference<
        "mutation",
        "internal",
        { cuisine?: string; query: string },
        any,
        Name
      >;
      fullTextSearchQuery: FunctionReference<
        "query",
        "internal",
        { cuisine?: string; query: string },
        any,
        Name
      >;
    };
    vectorActionV8: {
      vectorSearch: FunctionReference<
        "action",
        "internal",
        { cuisine: string; embedding: Array<number> },
        any,
        Name
      >;
    };
  };
