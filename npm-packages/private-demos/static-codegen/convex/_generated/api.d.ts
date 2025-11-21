/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type { FunctionReference } from "convex/server";
import type { GenericId as Id } from "convex/values";

/**
 * A utility for referencing Convex functions in your app's public API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = api.myModule.myFunction;
 * ```
 */
export declare const api: {
  messages: {
    componentTest: FunctionReference<"action", "public", {}, any>;
    list: FunctionReference<"query", "public", {}, any>;
    save: FunctionReference<"action", "public", { message: string }, string>;
    scheduleSendWaitlistMessage: FunctionReference<
      "mutation",
      "public",
      {},
      any
    >;
    send: FunctionReference<"mutation", "public", any, any>;
    testPartialRollback: FunctionReference<"mutation", "public", {}, any>;
  };
  staticFunctions: {
    a: FunctionReference<
      "action",
      "public",
      { branded: string; id: Id<"empty"> },
      string
    >;
    m: FunctionReference<
      "mutation",
      "public",
      { branded: string },
      { _creationTime: number; _id: Id<"empty"> } | null
    >;
    q: FunctionReference<"query", "public", { branded: string }, string>;
  };
};

/**
 * A utility for referencing Convex functions in your app's internal API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = internal.myModule.myFunction;
 * ```
 */
export declare const internal: {
  staticFunctions: {
    ia: FunctionReference<"action", "internal", {}, any>;
    im: FunctionReference<"mutation", "internal", { branded: string }, null>;
    iq: FunctionReference<
      "query",
      "internal",
      {
        arr: Array<number>;
        bool: boolean;
        data: ArrayBuffer;
        id: Id<"empty">;
        literal: "literal";
        null: null;
        num: number;
        str: string;
      },
      | string
      | "literal"
      | number
      | boolean
      | ArrayBuffer
      | Array<number>
      | null
      | Id<"empty">
    >;
  };
};

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
  staticComponent: {
    staticFunctions: {
      a: FunctionReference<
        "action",
        "internal",
        { branded: string; id: string },
        string
      >;
      m: FunctionReference<
        "mutation",
        "internal",
        { branded: string },
        { _creationTime: number; _id: string } | null
      >;
      q: FunctionReference<"query", "internal", { branded: string }, string>;
    };
  };
};
