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
import type * as errors from "../errors.js";
import type * as fileStorage from "../fileStorage.js";
import type * as functionHandles from "../functionHandles.js";
import type * as scheduler from "../scheduler.js";
import type * as transact from "../transact.js";

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
  errors: typeof errors;
  fileStorage: typeof fileStorage;
  functionHandles: typeof functionHandles;
  scheduler: typeof scheduler;
  transact: typeof transact;
}>;
export type Mounts = {
  cleanUp: {
    default: FunctionReference<"mutation", "public", any, any>;
  };
  errors: {
    throwConvexError: FunctionReference<"query", "public", any, any>;
    throwError: FunctionReference<"query", "public", any, any>;
  };
  fileStorage: {
    deleteById: FunctionReference<
      "mutation",
      "public",
      { storageId: string },
      any
    >;
    generateUploadUrl: FunctionReference<"mutation", "public", any, any>;
    get: FunctionReference<"query", "public", { id: string }, any>;
    getFile: FunctionReference<"action", "public", { storageId: string }, any>;
    getUrl: FunctionReference<"query", "public", { storageId: string }, any>;
    list: FunctionReference<"query", "public", any, any>;
    storeFile: FunctionReference<"action", "public", { data: string }, any>;
  };
  functionHandles: {
    fromAction: FunctionReference<"action", "public", any, any>;
    fromQuery: FunctionReference<"query", "public", any, any>;
    getInternalHandle: FunctionReference<
      "query",
      "public",
      { functionType: "query" | "mutation" | "action" },
      string
    >;
  };
  scheduler: {
    listAllMessages: FunctionReference<"query", "public", any, any>;
    scheduleWithinComponent: FunctionReference<
      "mutation",
      "public",
      { message: string },
      string
    >;
    sendMessage: FunctionReference<
      "mutation",
      "public",
      { message: string },
      any
    >;
    status: FunctionReference<"query", "public", { id: string }, any>;
  };
  transact: {
    allMessages: FunctionReference<"query", "public", {}, Array<string>>;
    sendButFail: FunctionReference<
      "mutation",
      "public",
      { message: string },
      any
    >;
    sendMessage: FunctionReference<
      "mutation",
      "public",
      { message: string },
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
