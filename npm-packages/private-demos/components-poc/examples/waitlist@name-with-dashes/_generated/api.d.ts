/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as actionDemo from "../actionDemo.js";
import type * as index from "../index.js";

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
  actionDemo: typeof actionDemo;
  index: typeof index;
}>;
export type Mounts = {
  actionDemo: {
    demo: FunctionReference<"action", "public", any, any>;
  };
  index: {
    fileDownloadUrl: FunctionReference<
      "query",
      "public",
      { id: string },
      string
    >;
    fileUploadUrl: FunctionReference<"mutation", "public", {}, string>;
    getMessageCount: FunctionReference<"query", "public", {}, number>;
    latestWrite: FunctionReference<"query", "public", {}, string>;
    listFiles: FunctionReference<"query", "public", any, any>;
    readFromFile: FunctionReference<"action", "public", { id: string }, string>;
    repeatMessage: FunctionReference<
      "action",
      "public",
      { message: string; n: number },
      string
    >;
    sayGoodbyeFromQuery: FunctionReference<"query", "public", {}, string>;
    sayHelloFromMutation: FunctionReference<"mutation", "public", {}, string>;
    scheduleMessage: FunctionReference<"mutation", "public", {}, any>;
    scheduleSend: FunctionReference<"mutation", "public", {}, any>;
    sendMessage: FunctionReference<"mutation", "public", {}, any>;
    storeInFile: FunctionReference<
      "action",
      "public",
      { message: string },
      string
    >;
    writeSuccessfully: FunctionReference<
      "mutation",
      "public",
      { text: string },
      any
    >;
    writeThenFail: FunctionReference<
      "mutation",
      "public",
      { text: string },
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

export declare const components: {
  ratelimiter: {
    index: {
      checkRateLimit: FunctionReference<
        "query",
        "internal",
        {
          count?: number;
          key?: string;
          name: string;
          reserve?: boolean;
          throws?: boolean;
        },
        { ok: boolean; retryAt?: number; ts?: number; value?: number }
      >;
      rateLimit: FunctionReference<
        "mutation",
        "internal",
        {
          count?: number;
          key?: string;
          name: string;
          reserve?: boolean;
          throws?: boolean;
        },
        { ok: boolean; retryAt?: number }
      >;
      resetRateLimit: FunctionReference<
        "mutation",
        "internal",
        { key?: string; name: string },
        any
      >;
    };
  };
};
