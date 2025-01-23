import { v } from "convex/values";
import { action } from "./_generated/server";
import { internal } from "./_generated/api";

export const doSomething = action({
  args: { a: v.number() },
  handler: async (ctx, args) => {
    const data = await ctx.runMutation(internal.myMutations.writeData, {
      a: args.a,
    });
    // do something else, optionally use `data`
  },
});
