/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as basic from "../basic.js";
import type * as conversations from "../conversations.js";
import type * as elle from "../elle.js";
import type * as messages from "../messages.js";
import type * as misc from "../misc.js";
import type * as sync_conversations from "../sync/conversations.js";
import type * as sync_messages from "../sync/messages.js";
import type * as sync_users from "../sync/users.js";
import type * as users from "../users.js";

import type {
  ApiFromModules,
  FilterApi,
  FunctionReference,
} from "convex/server";

declare const fullApi: ApiFromModules<{
  basic: typeof basic;
  conversations: typeof conversations;
  elle: typeof elle;
  messages: typeof messages;
  misc: typeof misc;
  "sync/conversations": typeof sync_conversations;
  "sync/messages": typeof sync_messages;
  "sync/users": typeof sync_users;
  users: typeof users;
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
