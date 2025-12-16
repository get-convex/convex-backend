import { mutation } from "./_generated/server";
import { api } from "./_generated/api";
import { v } from "convex/values";

export default mutation({
  args: {
    n: v.number(),
  },
  handler: async ({ db }, { n }) => {
    for (const message of await db.query("messages").take(n)) {
      await db.delete(message._id);
    }
  },
});

export const scheduleClearData = mutation({
  args: {
    n: v.optional(v.number()),
  },
  handler: async ({ scheduler }, { n = 10 }) => {
    await scheduler.runAfter(n * 1000, api.clearMessage.default, { n: 1 });
  },
});

export const doNothing = mutation({
  args: {},
  handler: () => {
    // intentional noop.
  },
});
