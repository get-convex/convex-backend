import { query } from "./_generated/server";

export const authInfo = query({
  args: {},
  handler: async (ctx) => {
    const userInfo = await ctx.auth.getUserIdentity();
    return userInfo;
  },
});
