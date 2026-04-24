import { jsonToConvex } from "../../values/index.js";
import {
  ActionMeta,
  MutationMeta,
  QueryMeta,
  RequestMetadata,
  FunctionMetadata,
  TransactionMetrics,
  DeploymentMetadata,
} from "../meta.js";
import { performAsyncSyscall } from "./syscall.js";

async function getTransactionMetrics(): Promise<TransactionMetrics> {
  let syscallJSON;
  try {
    syscallJSON = await performAsyncSyscall("1.0/getTransactionMetrics", {});
  } catch (e: any) {
    if (e.message?.includes("Unknown async operation")) {
      throw new Error(
        "getTransactionMetrics() can only be called from a query or mutation. " +
          "It is not available in actions or outside of a Convex function.",
      );
    }
    throw e;
  }
  return jsonToConvex(syscallJSON) as any;
}

async function getFunctionMetadata(): Promise<{
  name: string;
  componentPath: string;
}> {
  const { name, componentPath } = await performAsyncSyscall(
    "1.0/getFunctionMetadata",
    {},
  );
  return { name, componentPath };
}

async function getDeploymentMetadata(): Promise<DeploymentMetadata> {
  const syscallJSON = await performAsyncSyscall(
    "1.0/getDeploymentMetadata",
    {},
  );
  const result = jsonToConvex(syscallJSON) as any;
  return {
    name: result.name,
    region: result.region ?? null,
    class: result.class,
  };
}

async function getRequestMetadata(): Promise<RequestMetadata> {
  const { ip, userAgent, requestId } = await performAsyncSyscall(
    "1.0/getRequestMetadata",
    {},
  );
  return { ip, userAgent, requestId };
}

export function setupQueryMeta(
  visibility: FunctionMetadata["visibility"],
): QueryMeta {
  return {
    getFunctionMetadata: async () => ({
      ...(await getFunctionMetadata()),
      type: "query",
      visibility,
    }),
    getTransactionMetrics,
    getDeploymentMetadata,
  };
}

export function setupMutationMeta(
  visibility: FunctionMetadata["visibility"],
): MutationMeta {
  return {
    getFunctionMetadata: async () => ({
      ...(await getFunctionMetadata()),
      type: "mutation",
      visibility,
    }),
    getTransactionMetrics,
    getDeploymentMetadata,
    getRequestMetadata,
  };
}

export function setupActionMeta(
  visibility: FunctionMetadata["visibility"],
): ActionMeta {
  return {
    getFunctionMetadata: async () => ({
      ...(await getFunctionMetadata()),
      type: "action",
      visibility,
    }),
    getDeploymentMetadata,
    getRequestMetadata,
  };
}
