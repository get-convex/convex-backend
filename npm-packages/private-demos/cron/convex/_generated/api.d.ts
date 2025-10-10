/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as clearHighScore from "../clearHighScore.js";
import type * as clearMessage from "../clearMessage.js";
import type * as clearPresence from "../clearPresence.js";
import type * as crons from "../crons.js";
import type * as getTimes from "../getTimes.js";
import type * as listMessages from "../listMessages.js";
import type * as recordTime from "../recordTime.js";
import type * as sendEmail from "../sendEmail.js";
import type * as sendExpiringMessage from "../sendExpiringMessage.js";
import type * as sendMessage from "../sendMessage.js";

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
  clearHighScore: typeof clearHighScore;
  clearMessage: typeof clearMessage;
  clearPresence: typeof clearPresence;
  crons: typeof crons;
  getTimes: typeof getTimes;
  listMessages: typeof listMessages;
  recordTime: typeof recordTime;
  sendEmail: typeof sendEmail;
  sendExpiringMessage: typeof sendExpiringMessage;
  sendMessage: typeof sendMessage;
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
