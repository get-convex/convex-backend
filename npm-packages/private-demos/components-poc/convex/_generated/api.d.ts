/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as components_ from "../components.js";
import type * as crons from "../crons.js";
import type * as http from "../http.js";
import type * as messages from "../messages.js";
import type * as notes from "../notes.js";
import type * as triggers from "../triggers.js";
import type * as types from "../types.js";
import type * as withnode from "../withnode.js";

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
  components: typeof components_;
  crons: typeof crons;
  http: typeof http;
  messages: typeof messages;
  notes: typeof notes;
  triggers: typeof triggers;
  types: typeof types;
  withnode: typeof withnode;
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

export declare const components: {
  triggers: {
    documents: {
      deleteDoc: FunctionReference<
        "mutation",
        "internal",
        { atomicDelete: string; id: string; triggers: Array<string> },
        null
      >;
      insert: FunctionReference<
        "mutation",
        "internal",
        { atomicInsert: string; triggers: Array<string>; value: any },
        string
      >;
      patch: FunctionReference<
        "mutation",
        "internal",
        {
          atomicPatch: string;
          id: string;
          triggers: Array<string>;
          value: any;
        },
        null
      >;
      replace: FunctionReference<
        "mutation",
        "internal",
        {
          atomicReplace: string;
          id: string;
          triggers: Array<string>;
          value: any;
        },
        null
      >;
    };
  };
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
  ratelimiter: {
    index: {
      checkRateLimit: FunctionReference<
        "query",
        "internal",
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
        "internal",
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
        "internal",
        { key?: string; name: string },
        any
      >;
    };
  };
};
