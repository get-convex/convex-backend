import { api } from "./_generated/api";
import { action, query } from "./_generated/server";

// @ts-expect-error Circular type
// @snippet start tsError
// TypeScript reports an error on `myAction`
export const myAction = action({
  args: {},
  handler: async (ctx) => {
    return await ctx.runQuery(api.myFunctions.getSomething);
  },
});

export const getSomething = query({
  args: {},
  handler: () => {
    return null;
  },
});
// @snippet end tsError
