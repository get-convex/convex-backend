/* prettier-ignore-start */

/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as http from "../http.js";
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
  http: typeof http;
  index: typeof index;
}>;
export type Mounts = {
  index: {
    checkRateLimit: FunctionReference<
      "query",
      "public",
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
      "public",
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
      "public",
      { key?: string; name: string },
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

/* prettier-ignore-end */
