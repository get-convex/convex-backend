/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as crons from "../crons.js";
import type * as cursorRules from "../cursorRules.js";
import type * as http from "../http.js";
import type * as npm from "../npm.js";
import type * as util_convexClientHeader from "../util/convexClientHeader.js";
import type * as util_cursorRules from "../util/cursorRules.js";
import type * as util_github from "../util/github.js";
import type * as util_hash from "../util/hash.js";
import type * as util_isStale from "../util/isStale.js";
import type * as util_message from "../util/message.js";

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
  crons: typeof crons;
  cursorRules: typeof cursorRules;
  http: typeof http;
  npm: typeof npm;
  "util/convexClientHeader": typeof util_convexClientHeader;
  "util/cursorRules": typeof util_cursorRules;
  "util/github": typeof util_github;
  "util/hash": typeof util_hash;
  "util/isStale": typeof util_isStale;
  "util/message": typeof util_message;
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
