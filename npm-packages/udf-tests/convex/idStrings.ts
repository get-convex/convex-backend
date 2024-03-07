import { v } from "convex/values";
import { query } from "./_generated/server";

/**
 * Copied and pasted from syscall.ts
 * We don't normally allow UDFs to call ops, but this is for testing the system UDF `getTableMapping`
 **/
declare const Convex: {
  syscall: (op: string, jsonArgs: string) => string;
  asyncSyscall: (op: string, jsonArgs: string) => Promise<string>;
  jsSyscall: (op: string, args: Record<string, any>) => any;
  op: (opName: string, ...args: any[]) => any;
};

/**
 * Copied and pasted from syscall.ts
 * We don't normally allow UDFs to call ops, but this is for testing the system UDF `getTableMapping`
 **/
function performOp(op: string, ...args: any[]): any {
  if (typeof Convex === "undefined" || Convex.op === undefined) {
    throw new Error(
      "The Convex execution environment is being unexpectedly run outside of a Convex backend.",
    );
  }
  return Convex.op(op, ...args);
}

/**
 * Copied and pasted from dashboard/convex/_system/frontend,
 * so that it can be tested in UDF tests.
 */
export const getTableMapping = query(async () => {
  return performOp("getTableMapping");
});

export const normalizeId = query({
  args: { id: v.string(), table: v.string() },
  handler: ({ db }, { id, table }) => {
    const normalized = db.normalizeId(table as any, id);
    return { normalized };
  },
});

export const normalizeSystemId = query({
  args: { id: v.string(), table: v.string() },
  handler: ({ db }, { id, table }) => {
    const normalized = db.system.normalizeId(table as any, id);
    return normalized;
  },
});
