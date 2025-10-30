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

declare const fullApi: ApiFromModules<{
  cleanUp: typeof cleanUp;
  errors: typeof errors;
  fileStorage: typeof fileStorage;
  functionHandles: typeof functionHandles;
  scheduler: typeof scheduler;
  transact: typeof transact;
}>;

/**
 * A utility for referencing Convex functions in your app's public API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = api.myModule.myFunction;
 * ```
 */
export declare const api: FilterApi<
  typeof fullApi,
  FunctionReference<any, "public">
>;

/**
 * A utility for referencing Convex functions in your app's internal API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = internal.myModule.myFunction;
 * ```
 */
export declare const internal: FilterApi<
  typeof fullApi,
  FunctionReference<any, "internal">
>;

export declare const components: {};
