import { query } from "./_generated/server";
import { v } from "convex/values";

export const getNumber = query({
  args: { refreshKey: v.optional(v.number()) },
  handler: async (ctx, args) => {
    const userIdentity = await ctx.auth.getUserIdentity();
    const userIdentityInsecure = await ctx.auth.getUserIdentityInsecure();
    const userIdentityDebug = await ctx.auth.getUserIdentityDebug();

    console.log("getUserIdentity():", userIdentity);
    console.log("getUserIdentityInsecure():", userIdentityInsecure);
    console.log("getUserIdentityDebug():", userIdentityDebug);
    console.log("Query executed with refreshKey:", args.refreshKey);

    return {
      number: 100,
      userIdentity,
      userIdentityInsecure,
      userIdentityDebug,
      refreshKey: args.refreshKey,
      magic: userIdentityInsecure == "asdf",
    };
  },
});
