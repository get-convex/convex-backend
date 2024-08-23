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
declare const fullApiWithMounts: typeof fullApi & {
  messages: {
    envVarAction: FunctionReference<"action", "public", any, any>;
    envVarQuery: FunctionReference<"query", "public", any, any>;
    hello: FunctionReference<"action", "public", any, any>;
    systemEnvVarAction: FunctionReference<"action", "public", any, any>;
    systemEnvVarQuery: FunctionReference<"query", "public", any, any>;
    url: FunctionReference<"action", "public", any, any>;
  };
};

export declare const api: FilterApi<
  typeof fullApiWithMounts,
  FunctionReference<any, "public">
>;
export declare const internal: FilterApi<
  typeof fullApiWithMounts,
  FunctionReference<any, "internal">
>;
