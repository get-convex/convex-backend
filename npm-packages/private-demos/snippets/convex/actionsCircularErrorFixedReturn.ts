import { api } from "./_generated/api";
import { action } from "./_generated/server";

// @snippet start fixed
export const myAction = action({
  args: {},
  handler: async (ctx): Promise<null> => {
    const result = await ctx.runQuery(api.myFunctions.getSomething);
    return result;
  },
});
// @snippet end fixed
