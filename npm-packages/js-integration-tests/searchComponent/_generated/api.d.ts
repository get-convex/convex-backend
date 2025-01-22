/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as cleanUp from "../cleanUp.js";
import type * as foods from "../foods.js";
import type * as textSearch from "../textSearch.js";
import type * as vectorActionV8 from "../vectorActionV8.js";

import type {
  ApiFromModules,
  FilterApi,
  FunctionReference,
} from "convex/server";
/**
 * A utility for referencing Convex functions in your app's API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = api.myModule.myFunction;
 * ```
 */
declare const fullApi: ApiFromModules<{
  cleanUp: typeof cleanUp;
  foods: typeof foods;
  textSearch: typeof textSearch;
  vectorActionV8: typeof vectorActionV8;
}>;
export type Mounts = {
  cleanUp: {
    default: FunctionReference<"mutation", "public", any, any>;
  };
  foods: {
    insertRow: FunctionReference<
      "mutation",
      "public",
      { cuisine: string; description: string; embedding: Array<number> },
      any
    >;
    populate: FunctionReference<"action", "public", {}, any>;
    queryDocs: FunctionReference<
      "query",
      "public",
      { ids: Array<string> },
      any
    >;
  };
  textSearch: {
    fullTextSearchMutation: FunctionReference<
      "mutation",
      "public",
      { cuisine?: string; query: string },
      any
    >;
    fullTextSearchMutationWithWrite: FunctionReference<
      "mutation",
      "public",
      { cuisine?: string; query: string },
      any
    >;
    fullTextSearchQuery: FunctionReference<
      "query",
      "public",
      { cuisine?: string; query: string },
      any
    >;
  };
  vectorActionV8: {
    vectorSearch: FunctionReference<
      "action",
      "public",
      { cuisine: string; embedding: Array<number> },
      any
    >;
  };
};
// For now fullApiWithMounts is only fullApi which provides
// jump-to-definition in component client code.
// Use Mounts for the same type without the inference.
declare const fullApiWithMounts: typeof fullApi;

export declare const api: FilterApi<
  typeof fullApiWithMounts,
  FunctionReference<any, "public">
>;
export declare const internal: FilterApi<
  typeof fullApiWithMounts,
  FunctionReference<any, "internal">
>;

export declare const components: {};
