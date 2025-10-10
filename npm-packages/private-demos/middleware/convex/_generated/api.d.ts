/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as lib_dependentMiddlewareTemplate from "../lib/dependentMiddlewareTemplate.js";
import type * as lib_mergeMiddlewareTemplate from "../lib/mergeMiddlewareTemplate.js";
import type * as lib_middlewareTemplate from "../lib/middlewareTemplate.js";
import type * as lib_middlewareUtils from "../lib/middlewareUtils.js";
import type * as lib_relationships from "../lib/relationships.js";
import type * as lib_rowLevelSecurity from "../lib/rowLevelSecurity.js";
import type * as lib_withReplacer from "../lib/withReplacer.js";
import type * as lib_withSession from "../lib/withSession.js";
import type * as lib_withUser from "../lib/withUser.js";
import type * as sessions from "../sessions.js";

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
  "lib/dependentMiddlewareTemplate": typeof lib_dependentMiddlewareTemplate;
  "lib/mergeMiddlewareTemplate": typeof lib_mergeMiddlewareTemplate;
  "lib/middlewareTemplate": typeof lib_middlewareTemplate;
  "lib/middlewareUtils": typeof lib_middlewareUtils;
  "lib/relationships": typeof lib_relationships;
  "lib/rowLevelSecurity": typeof lib_rowLevelSecurity;
  "lib/withReplacer": typeof lib_withReplacer;
  "lib/withSession": typeof lib_withSession;
  "lib/withUser": typeof lib_withUser;
  sessions: typeof sessions;
}>;
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
