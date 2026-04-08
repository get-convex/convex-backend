import { jsonToConvex } from "../../values/index.js";
import {
  ActionMeta,
  MutationMeta,
  QueryMeta,
  TransactionMetrics,
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

export function setupQueryMeta(): QueryMeta {
  return {
    getTransactionMetrics,
  };
}

export function setupMutationMeta(): MutationMeta {
  return {
    getTransactionMetrics,
  };
}

export function setupActionMeta(): ActionMeta {
  return {};
}
