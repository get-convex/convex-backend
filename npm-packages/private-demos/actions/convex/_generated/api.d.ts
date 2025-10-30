/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as cancelJob from "../cancelJob.js";
import type * as dangle from "../dangle.js";
import type * as email from "../email.js";
import type * as http from "../http.js";
import type * as langchain from "../langchain.js";
import type * as listMessages from "../listMessages.js";
import type * as node from "../node.js";
import type * as nodeAction from "../nodeAction.js";
import type * as notSourceMappable from "../notSourceMappable.js";
import type * as sendGifMessage from "../sendGifMessage.js";
import type * as sendMessage from "../sendMessage.js";
import type * as simple from "../simple.js";
import type * as sourceMappable from "../sourceMappable.js";
import type * as tac from "../tac.js";
import type * as tic from "../tic.js";
import type * as toe from "../toe.js";
import type * as virtualTable from "../virtualTable.js";

import type {
  ApiFromModules,
  FilterApi,
  FunctionReference,
} from "convex/server";

declare const fullApi: ApiFromModules<{
  cancelJob: typeof cancelJob;
  dangle: typeof dangle;
  email: typeof email;
  http: typeof http;
  langchain: typeof langchain;
  listMessages: typeof listMessages;
  node: typeof node;
  nodeAction: typeof nodeAction;
  notSourceMappable: typeof notSourceMappable;
  sendGifMessage: typeof sendGifMessage;
  sendMessage: typeof sendMessage;
  simple: typeof simple;
  sourceMappable: typeof sourceMappable;
  tac: typeof tac;
  tic: typeof tic;
  toe: typeof toe;
  virtualTable: typeof virtualTable;
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
