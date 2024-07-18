import { convexToJson, jsonToConvex, Value } from "../../values/index.js";
import { version } from "../../index.js";
import { performAsyncSyscall } from "./syscall.js";
import { parseArgs } from "../../common/index.js";
import { functionName, FunctionReference } from "../../server/api.js";
import { extractReferencePath } from "../components/reference.js";

function syscallArgs(
  requestId: string,
  functionReference: any,
  args?: Record<string, Value>,
) {
  const address = getFunctionAddress(functionReference);
  return {
    ...address,
    args: convexToJson(parseArgs(args)),
    version,
    requestId,
  };
}

export function getFunctionAddress(functionReference: any) {
  // The `run*` syscalls expect either a UDF path at "name" or a serialized
  // reference at "reference". Dispatch on `functionReference` to coerce
  // it to one ore the other.
  let functionAddress;

  // Legacy path for passing in UDF paths directly as function references.
  if (typeof functionReference === "string") {
    functionAddress = { name: functionReference };
  }
  // Path for passing in a `FunctionReference`, either from `api` or directly
  // created from a UDF path with `makeFunctionReference`.
  else if (functionReference[functionName]) {
    functionAddress = { name: functionReference[functionName] };
  }
  // Reference to a component's function derived from `app` or `component`.
  else {
    const referencePath = extractReferencePath(functionReference);
    if (!referencePath) {
      throw new Error(`${functionReference} is not a functionReference`);
    }
    functionAddress = { reference: referencePath };
  }
  return functionAddress;
}

export function setupActionCalls(requestId: string) {
  return {
    runQuery: async (
      query: FunctionReference<"query", "public" | "internal">,
      args?: Record<string, Value>,
    ): Promise<any> => {
      const result = await performAsyncSyscall(
        "1.0/actions/query",
        syscallArgs(requestId, query, args),
      );
      return jsonToConvex(result);
    },
    runMutation: async (
      mutation: FunctionReference<"mutation", "public" | "internal">,
      args?: Record<string, Value>,
    ): Promise<any> => {
      const result = await performAsyncSyscall(
        "1.0/actions/mutation",
        syscallArgs(requestId, mutation, args),
      );
      return jsonToConvex(result);
    },
    runAction: async (
      action: FunctionReference<"action", "public" | "internal">,
      args?: Record<string, Value>,
    ): Promise<any> => {
      const result = await performAsyncSyscall(
        "1.0/actions/action",
        syscallArgs(requestId, action, args),
      );
      return jsonToConvex(result);
    },
  };
}
