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
    staticFunctions: {
      a: FunctionReference<
        "action",
        "internal",
        { branded: string; id: string },
        string,
        Name
      >;
      m: FunctionReference<
        "mutation",
        "internal",
        { branded: string },
        { _creationTime: number; _id: string } | null,
        Name
      >;
      q: FunctionReference<
        "query",
        "internal",
        { branded: string },
        string,
        Name
      >;
    };
  };
