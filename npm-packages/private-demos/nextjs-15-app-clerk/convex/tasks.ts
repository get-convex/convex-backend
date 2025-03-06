import { query } from "./_generated/server";

export const get = query({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.query("tasks").collect();
  },
});

export const user = query({
  handler: async (ctx) => {
    return await ctx.auth.getUserIdentity();
  },
});
