import { api } from "./_generated/api";
import { query, mutation } from "./_generated/server";

export const q = query({
  args: {},
  handler: async ({ auth }) => {
    return await auth.getUserIdentity();
  },
});

export const m = mutation({
  args: {},
  handler: async ({ auth }) => {
    return await auth.getUserIdentity();
  },
});

export const s = mutation({
  args: {},
  handler: async ({ scheduler, auth }) => {
    if (!auth.getUserIdentity()) {
      throw new Error("not authed");
    }
    await scheduler.runAfter(0, api.actions.auth.storeUser);
  },
});
