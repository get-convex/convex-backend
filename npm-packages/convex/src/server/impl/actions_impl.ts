import { convexToJson, jsonToConvex, Value } from "../../values/index.js";
import { version } from "../../index.js";
import { performAsyncSyscall } from "./syscall.js";
import { parseArgs } from "../../common/index.js";
import { FunctionReference, getFunctionName } from "../../server/api.js";

export function setupActionCalls(requestId: string) {
  return {
    runQuery: async (
      query: FunctionReference<"query", "public" | "internal">,
      args?: Record<string, Value>,
    ): Promise<any> => {
      const name = getFunctionName(query);
      const queryArgs = parseArgs(args);
      const syscallArgs = {
        name,
        args: convexToJson(queryArgs),
        version,
        requestId,
      };
      const result = await performAsyncSyscall(
        "1.0/actions/query",
        syscallArgs,
      );
      return jsonToConvex(result);
    },
    runMutation: async (
      mutation: FunctionReference<"mutation", "public" | "internal">,
      args?: Record<string, Value>,
    ): Promise<any> => {
      const name = getFunctionName(mutation);
      const mutationArgs = parseArgs(args);
      const syscallArgs = {
        name,
        args: convexToJson(mutationArgs),
        version,
        requestId,
      };
      const result = await performAsyncSyscall(
        "1.0/actions/mutation",
        syscallArgs,
      );
      return jsonToConvex(result);
    },
    runAction: async (
      action: FunctionReference<"action", "public" | "internal">,
      args?: Record<string, Value>,
    ): Promise<any> => {
      const name = getFunctionName(action);
      const actionArgs = parseArgs(args);
      const syscallArgs = {
        name,
        args: convexToJson(actionArgs),
        version,
        requestId,
      };
      const result = await performAsyncSyscall(
        "1.0/actions/action",
        syscallArgs,
      );
      return jsonToConvex(result);
    },
  };
}
