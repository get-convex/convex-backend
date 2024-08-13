/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as throwSystemError from "../throwSystemError.js";

import type { ApiFromModules, FunctionReference } from "convex/server";
/**
 * A utility for referencing Convex functions in your app's API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = functions.myModule.myFunction;
 * ```
 */
declare const functions: ApiFromModules<{
  throwSystemError: typeof throwSystemError;
}>;
