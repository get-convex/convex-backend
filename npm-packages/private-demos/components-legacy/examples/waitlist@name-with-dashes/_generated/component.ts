/* eslint-disable */
/**
 * Generated `ComponentApi` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type { FunctionReference } from "convex/server";

/**
 * A utility for referencing a Convex component's exposed API.
 *
 * Useful when expecting a parameter like `components.myComponent`.
 * Usage:
 * ```ts
 * async function myFunction(ctx: QueryCtx, component: ComponentApi) {
 *   return ctx.runQuery(component.someFile.someQuery, { ...args });
 * }
 * ```
 */
export type ComponentApi<Name extends string | undefined = string | undefined> =
  {
    actionDemo: {
      demo: FunctionReference<"action", "internal", any, any, Name>;
    };
    index: {
      fileDownloadUrl: FunctionReference<
        "query",
        "internal",
        { id: string },
        string,
        Name
      >;
      fileUploadUrl: FunctionReference<
        "mutation",
        "internal",
        {},
        string,
        Name
      >;
      getMessageCount: FunctionReference<"query", "internal", {}, number, Name>;
      latestWrite: FunctionReference<"query", "internal", {}, string, Name>;
      listFiles: FunctionReference<"query", "internal", any, any, Name>;
      readFromFile: FunctionReference<
        "action",
        "internal",
        { id: string },
        string,
        Name
      >;
      repeatMessage: FunctionReference<
        "action",
        "internal",
        { message: string; n: number },
        string,
        Name
      >;
      sayGoodbyeFromQuery: FunctionReference<
        "query",
        "internal",
        {},
        string,
        Name
      >;
      sayHelloFromMutation: FunctionReference<
        "mutation",
        "internal",
        {},
        string,
        Name
      >;
      scheduleMessage: FunctionReference<"mutation", "internal", {}, any, Name>;
      scheduleSend: FunctionReference<"mutation", "internal", {}, any, Name>;
      sendMessage: FunctionReference<"mutation", "internal", {}, any, Name>;
      storeInFile: FunctionReference<
        "action",
        "internal",
        { message: string },
        string,
        Name
      >;
      writeSuccessfully: FunctionReference<
        "mutation",
        "internal",
        { text: string },
        any,
        Name
      >;
      writeThenFail: FunctionReference<
        "mutation",
        "internal",
        { text: string },
        any,
        Name
      >;
    };
  };
