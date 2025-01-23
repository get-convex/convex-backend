import { api } from "./_generated/api";
import { mutation } from "./_generated/server";

export const scheduleMessage = mutation({
  args: {},
  handler: async (ctx) => {
    await ctx.scheduler.runAfter(0, api.insert.insertMessageWithSearch, {});
  },
});
