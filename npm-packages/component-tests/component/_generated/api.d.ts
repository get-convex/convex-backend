/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as messages from "../messages.js";

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
  messages: typeof messages;
}>;
export type Mounts = {
  messages: {
    dateNow: FunctionReference<"query", "public", {}, any>;
    hello: FunctionReference<"action", "public", {}, any>;
    insertMessage: FunctionReference<
      "mutation",
      "public",
      { channel: string; text: string },
      any
    >;
    listMessages: FunctionReference<"query", "public", {}, any>;
    mathRandom: FunctionReference<"query", "public", {}, any>;
    tryToPaginate: FunctionReference<"query", "public", {}, any>;
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
