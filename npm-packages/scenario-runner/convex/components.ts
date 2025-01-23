import { components } from "./_generated/api";
import { action, mutation, query } from "./_generated/server";

export const componentQuery = query({
  handler: async (ctx) => {
    return await ctx.runQuery(components.counterComponent.public.load);
  },
});

export const componentMutation = mutation({
  handler: async (ctx) => {
    return await ctx.runMutation(components.counterComponent.public.increment);
  },
});

export const componentAction = action({
  handler: async (ctx) => {
    const count = Math.floor(Math.random() * 4) + 1;
    await ctx.runAction(components.counterComponent.public.reset, { count });
    await ctx.runMutation(components.counterComponent.public.increment);
    return await ctx.runQuery(components.counterComponent.public.load);
  },
});
