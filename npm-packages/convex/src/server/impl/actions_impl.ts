import { convexToJson, jsonToConvex, Value } from "../../values/index.js";
import { version } from "../../index.js";
import { performAsyncSyscall } from "./syscall.js";
import { parseArgs } from "../../common/index.js";
import { FunctionReference } from "../../server/api.js";
import { getFunctionAddress } from "../components/paths.js";
import { validateArg } from "./validate.js";

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

/**
 * Get a short-lived credential for calling a Convex-managed service.
 *
 * This function can only be called while an action is running. The credential
 * is scoped to the current deployment and should be sent as a bearer token.
 * Repeated calls in the same action reuse one token; a failed mint is not
 * cached, so a later call retries.
 *
 * @param service - The service the credential may access.
 * @internal
 */
export async function getServiceToken(service: "ai-gateway"): Promise<string> {
  validateArg(service, 1, "getServiceToken", "service");
  if (service !== "ai-gateway") {
    throw new Error(`Unsupported service "${String(service)}"`);
  }
  return await performAsyncSyscall("1.0/createServiceToken", {
    service,
    version,
  });
}
