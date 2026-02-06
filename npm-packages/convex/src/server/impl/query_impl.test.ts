import { QueryImpl } from "./query_impl.js";
import { test, expect } from "vitest";

// Mock to prevent
// "The Convex database and auth objects are being used outside of a Convex backend..." errors
(globalThis as any).Convex = {
  syscall: (_op: string, _jsonArgs: string) => {
    return "{}";
  },
  asyncSyscall: async (_op: string, _jsonArgs: string) => {
    return new Promise((resolve) => {
      resolve('{ "done": true, "value": null }');
    });
  },
};

function newQuery() {
  return new QueryImpl({
    source: {
      type: "FullTableScan",
      tableName: "messages",
      order: null,
    },
    operators: [],
  });
}

test("take does not throw if passed a non-negative integer", async () => {
  await newQuery().take(1);
});

test("take throws a TypeError if passed a float", async () => {
  const t = () => {
    return newQuery().take(1.5);
  };
  await expect(t).rejects.toThrow(TypeError);
  await expect(t).rejects.toThrow(/must be a non-negative integer/);
});

test("take throws a TypeError if passed a negative integer", async () => {
  const t = () => {
    return newQuery().take(-1);
  };
  await expect(t).rejects.toThrow(TypeError);
  await expect(t).rejects.toThrow(/must be a non-negative integer/);
});

test("filter(undefined) does not add a filter operator to the query", async () => {
  let capturedQuery: { operators: unknown[] } | null = null;
  const originalSyscall = (globalThis as any).Convex.syscall;
  (globalThis as any).Convex.syscall = (op: string, jsonArgs: string) => {
    if (op === "1.0/queryStream") {
      const arg = JSON.parse(jsonArgs);
      capturedQuery = arg.query;
    }
    return originalSyscall(op, jsonArgs);
  };
  try {
    await newQuery().filter(() => undefined).collect();
    expect(capturedQuery).not.toBeNull();
    expect(capturedQuery!.operators).toHaveLength(0);
  } finally {
    (globalThis as any).Convex.syscall = originalSyscall;
  }
});

test("filter(undefined) then filter(expr) adds only one filter operator", async () => {
  let capturedQuery: { operators: unknown[] } | null = null;
  const originalSyscall = (globalThis as any).Convex.syscall;
  (globalThis as any).Convex.syscall = (op: string, jsonArgs: string) => {
    if (op === "1.0/queryStream") {
      const arg = JSON.parse(jsonArgs);
      capturedQuery = arg.query;
    }
    return originalSyscall(op, jsonArgs);
  };
  try {
    await newQuery()
      .filter(() => undefined)
      .filter((q) => q.eq(q.field("body"), "x"))
      .collect();
    expect(capturedQuery).not.toBeNull();
    expect(capturedQuery!.operators).toHaveLength(1);
    expect(capturedQuery!.operators[0]).toHaveProperty("filter");
  } finally {
    (globalThis as any).Convex.syscall = originalSyscall;
  }
});
