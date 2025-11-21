import { action, mutation, query } from "./_generated/server";

export const insertValue = mutation({
  args: {},
  handler: async ({ db }) => {
    await db.insert("sets", { set: new Set([1, 2]) });
  },
});

export const listValues = query({
  args: {},
  handler: async () => {
    return new Set();
  },
});

export const mutationReturningSet = mutation({
  args: {},
  handler: async () => {
    return new Set(["hello", "world"]);
  },
});

export const actionReturningSet = action({
  args: {},
  handler: async () => {
    return new Set(["hello", "world"]);
  },
});

export const queryWithAnyArg = query({
  handler: async (_, _args: { x: any }) => {
    // Do nada
  },
});
export const mutationWithAnyArg = mutation({
  handler: async (_, _args: { x: any }) => {
    // Do nada
  },
});
export const actionWithAnyArg = action({
  handler: async (_, _args: { x: any }) => {
    // Do nada
  },
});
