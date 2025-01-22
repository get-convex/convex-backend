import { mutation } from "./_generated/server";

export const myMutation = mutation({
  args: {
    // ...
  },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    const { tokenIdentifier, name, email } = identity;
    //...
  },
});
