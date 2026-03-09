import { jsonToConvex } from "../values/index.js";
import { performAsyncSyscall } from "./impl/syscall.js";

/**
 * Used and remaining amounts for a single transaction limit.
 *
 * @public
 */
export type TransactionMetric = {
  used: number;
  remaining: number;
};

/**
 * The remaining headroom for a transaction before hitting limits.
 *
 * See https://docs.convex.dev/production/state/limits
 *
 * @public
 */
export type TransactionHeadroom = {
  bytesRead: TransactionMetric;
  bytesWritten: TransactionMetric;
  databaseQueries: TransactionMetric;
  documentsRead: TransactionMetric;
  documentsWritten: TransactionMetric;
  functionsScheduled: TransactionMetric;
  scheduledFunctionArgsBytes: TransactionMetric;
};

/**
 * Get the remaining headroom for the current transaction before hitting limits.
 *
 * @returns An object containing the remaining capacity for reads, writes, and scheduled functions.
 * @public
 */
export async function getTransactionHeadroom(): Promise<TransactionHeadroom> {
  let syscallJSON;
  try {
    syscallJSON = await performAsyncSyscall("1.0/headroom", {});
  } catch (e: any) {
    if (e.message?.includes("Unknown async operation")) {
      throw new Error(
        "getTransactionHeadroom() can only be called from a query or mutation. " +
          "It is not available in actions or outside of a Convex function.",
      );
    }
    throw e;
  }
  return jsonToConvex(syscallJSON) as any;
}
