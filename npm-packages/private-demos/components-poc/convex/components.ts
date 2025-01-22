import { api } from "./_generated/api";
import { mutation } from "./_generated/server";

export const foo = mutation({
  args: {},
  handler: async (ctx) => {
    // Regression test: making sure `api.components` works and refers to the
    // file `components.ts`, not to installed components.
    await ctx.scheduler.runAfter(1000, api.components.foo);
  },
});
