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
    envVarAction: FunctionReference<"action", "public", any, any>;
    envVarQuery: FunctionReference<"query", "public", any, any>;
    hello: FunctionReference<"action", "public", any, any>;
    systemEnvVarAction: FunctionReference<"action", "public", any, any>;
    systemEnvVarQuery: FunctionReference<"query", "public", any, any>;
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

export declare const components: {
  component: {
    messages: {
      dateNow: FunctionReference<"query", "internal", {}, any>;
      hello: FunctionReference<"action", "internal", {}, any>;
      insertMessage: FunctionReference<
        "mutation",
        "internal",
        { channel: string; text: string },
        any
      >;
      listMessages: FunctionReference<"query", "internal", {}, any>;
      mathRandom: FunctionReference<"query", "internal", {}, any>;
      tryToPaginate: FunctionReference<"query", "internal", {}, any>;
    };
  };
};
