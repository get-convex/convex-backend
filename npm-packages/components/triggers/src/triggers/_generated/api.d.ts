/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as documents from "../documents.js";

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
  documents: typeof documents;
}>;
export type Mounts = {
  documents: {
    deleteDoc: FunctionReference<
      "mutation",
      "public",
      { atomicDelete: string; id: string; triggers: Array<string> },
      null
    >;
    insert: FunctionReference<
      "mutation",
      "public",
      { atomicInsert: string; triggers: Array<string>; value: any },
      string
    >;
    patch: FunctionReference<
      "mutation",
      "public",
      { atomicPatch: string; id: string; triggers: Array<string>; value: any },
      null
    >;
    replace: FunctionReference<
      "mutation",
      "public",
      {
        atomicReplace: string;
        id: string;
        triggers: Array<string>;
        value: any;
      },
      null
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
