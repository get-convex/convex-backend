import { query } from "./_generated/server";

export const getCurrentUserIdentity = query({
  args: {},
  handler: async (ctx) => {
    const userIdentity = await ctx.auth.getUserIdentityDebug();
    console.log("Current user identity:", userIdentity);
    return userIdentity;
  },
});
