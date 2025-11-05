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
import { anyApi, componentsGeneric } from "convex/server";

const fullApi: ApiFromModules<{
  messages: typeof messages;
}> = anyApi as any;

/**
 * A utility for referencing Convex functions in your app's public API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = api.myModule.myFunction;
 * ```
 */
export const api: FilterApi<
  typeof fullApi,
  FunctionReference<any, "public">
> = anyApi as any;

/**
 * A utility for referencing Convex functions in your app's internal API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = internal.myModule.myFunction;
 * ```
 */
export const internal: FilterApi<
  typeof fullApi,
  FunctionReference<any, "internal">
> = anyApi as any;

export const components = componentsGeneric() as unknown as {
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
