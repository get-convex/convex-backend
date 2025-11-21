import { Doc } from "./_generated/dataModel";
import { mutation, query } from "./_generated/server";

export const returnsResolved = query({
  args: {},
  handler: async () => {
    return await Promise.resolve("hello world");
  },
});

export const neverResolves = query({
  args: {},
  handler: async () => {
    return new Promise((_resolve) => {
      // never call resolve()
    });
  },
});

export const danglingMutation = mutation({
  args: {},
  handler: async ({ db }) => {
    // eslint-disable-next-line @typescript-eslint/no-floating-promises
    db.insert("test", {});
  },
});

export const doublyDanglingMutation = mutation({
  args: {},
  handler: async ({ db }) => {
    const twoMutations = async () => {
      await db.insert("test", {});
      await db.insert("test", {});
    };
    void twoMutations();
  },
});

export const queryTestTable = query({
  args: {},
  handler: async ({ db }) => {
    return db.query("test").collect();
  },
});

// We run all syscalls to completion and drain the microtask queue
// before reading the query result. However, the result is serialized
// to json before the final drain, so dangling async code cannot
// affect the result.
export const queryDangling = query({
  args: {},
  handler: async ({ db }) => {
    const result: (Doc<"objects"> | null)[] = [];
    void db
      .query("objects")
      .first()
      .then((d) => result.push(d));
    return result;
  },
});

export const syscallAfterDanglingSyscall = mutation({
  args: {},
  handler: async ({ db }) => {
    // Not awaited.
    // eslint-disable-next-line @typescript-eslint/no-floating-promises
    db.insert("test", {}).then(() => {
      Date.now();
    });
  },
});
