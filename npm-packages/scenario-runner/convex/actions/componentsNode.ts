"use node";

import { components } from "../_generated/api";
import { action } from "../_generated/server";

export const callComponentFromNodeAction = action({
  handler: async (ctx) => {
    const count = Math.floor(Math.random() * 4) + 1;
    await ctx.runAction(components.counterComponent.public.reset, { count });
    await ctx.runMutation(components.counterComponent.public.increment);
    return await ctx.runQuery(components.counterComponent.public.load);
  },
});
