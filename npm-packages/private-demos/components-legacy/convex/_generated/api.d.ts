/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as messages from "../messages.js";

import type {
  ApiFromModules,
  FilterApi,
  FunctionReference,
} from "convex/server";

declare const fullApi: ApiFromModules<{
  messages: typeof messages;
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

export declare const components: {
  waitlist: {
    actionDemo: {
      demo: FunctionReference<"action", "internal", any, any>;
    };
    index: {
      fileDownloadUrl: FunctionReference<
        "query",
        "internal",
        { id: string },
        string
      >;
      fileUploadUrl: FunctionReference<"mutation", "internal", {}, string>;
      getMessageCount: FunctionReference<"query", "internal", {}, number>;
      latestWrite: FunctionReference<"query", "internal", {}, string>;
      listFiles: FunctionReference<"query", "internal", any, any>;
      readFromFile: FunctionReference<
        "action",
        "internal",
        { id: string },
        string
      >;
      repeatMessage: FunctionReference<
        "action",
        "internal",
        { message: string; n: number },
        string
      >;
      sayGoodbyeFromQuery: FunctionReference<"query", "internal", {}, string>;
      sayHelloFromMutation: FunctionReference<
        "mutation",
        "internal",
        {},
        string
      >;
      scheduleMessage: FunctionReference<"mutation", "internal", {}, any>;
      scheduleSend: FunctionReference<"mutation", "internal", {}, any>;
      sendMessage: FunctionReference<"mutation", "internal", {}, any>;
      storeInFile: FunctionReference<
        "action",
        "internal",
        { message: string },
        string
      >;
      writeSuccessfully: FunctionReference<
        "mutation",
        "internal",
        { text: string },
        any
      >;
      writeThenFail: FunctionReference<
        "mutation",
        "internal",
        { text: string },
        any
      >;
    };
  };
  waitlist2: {
    actionDemo: {
      demo: FunctionReference<"action", "internal", any, any>;
    };
    index: {
      fileDownloadUrl: FunctionReference<
        "query",
        "internal",
        { id: string },
        string
      >;
      fileUploadUrl: FunctionReference<"mutation", "internal", {}, string>;
      getMessageCount: FunctionReference<"query", "internal", {}, number>;
      latestWrite: FunctionReference<"query", "internal", {}, string>;
      listFiles: FunctionReference<"query", "internal", any, any>;
      readFromFile: FunctionReference<
        "action",
        "internal",
        { id: string },
        string
      >;
      repeatMessage: FunctionReference<
        "action",
        "internal",
        { message: string; n: number },
        string
      >;
      sayGoodbyeFromQuery: FunctionReference<"query", "internal", {}, string>;
      sayHelloFromMutation: FunctionReference<
        "mutation",
        "internal",
        {},
        string
      >;
      scheduleMessage: FunctionReference<"mutation", "internal", {}, any>;
      scheduleSend: FunctionReference<"mutation", "internal", {}, any>;
      sendMessage: FunctionReference<"mutation", "internal", {}, any>;
      storeInFile: FunctionReference<
        "action",
        "internal",
        { message: string },
        string
      >;
      writeSuccessfully: FunctionReference<
        "mutation",
        "internal",
        { text: string },
        any
      >;
      writeThenFail: FunctionReference<
        "mutation",
        "internal",
        { text: string },
        any
      >;
    };
  };
};
