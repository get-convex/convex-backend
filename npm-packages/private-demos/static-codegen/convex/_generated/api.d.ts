/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type { FunctionReference } from "convex/server";
import type { GenericId as Id } from "convex/values";

/**
 * A utility for referencing Convex functions in your app's public API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = api.myModule.myFunction;
 * ```
 */
export declare const api: {
  staticFunctions: {
    q: FunctionReference<
      "query",
      "public",
      { branded: string; id: Id<"empty"> },
      { _creationTime: number; _id: Id<"empty"> } | string
    >;
    m: FunctionReference<
      "mutation",
      "public",
      { branded: string; id: Id<"empty"> },
      string
    >;
    a: FunctionReference<
      "action",
      "public",
      { branded: string; id: Id<"empty"> },
      string
    >;
  };
};

/**
 * A utility for referencing Convex functions in your app's internal API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = internal.myModule.myFunction;
 * ```
 */
export declare const internal: {
  staticFunctions: {
    iq: FunctionReference<
      "query",
      "internal",
      {
        arr: Array<number>;
        bool: boolean;
        data: ArrayBuffer;
        id: Id<"empty">;
        literal: "literal";
        null: null;
        num: number;
        str: string;
      },
      | string
      | "literal"
      | number
      | boolean
      | ArrayBuffer
      | Array<number>
      | null
      | Id<"empty">
    >;
    im: FunctionReference<"mutation", "internal", { branded: string }, null>;
    ia: FunctionReference<"action", "internal", {}, any>;
  };
};

export declare const components: {};
