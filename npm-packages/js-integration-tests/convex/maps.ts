import { action, query, mutation } from "./_generated/server";

export const createMap = mutation({
  args: {},
  handler: async ({ db }) => {
    return await db.insert("maps", { map: new Map([["n", "m"]]) });
  },
});

export const listValues = query({
  args: {},
  handler: async () => {
    return new Map();
  },
});

export const mutationReturningMap = mutation({
  args: {},
  handler: async () => {
    return new Map([["key", "value"]]);
  },
});

export const actionReturningMap = action({
  args: {},
  handler: async () => {
    return new Map([["key", "value"]]);
  },
});
