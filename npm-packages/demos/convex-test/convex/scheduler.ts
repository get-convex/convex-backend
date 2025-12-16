import { v } from "convex/values";
import { api, internal } from "./_generated/api";
import { Id } from "./_generated/dataModel";
import { internalAction, mutation } from "./_generated/server";

export const mutationSchedulingAction = mutation({
  args: {
    delayMs: v.number(),
    createTask: v.optional(v.boolean()),
  },
  handler: async (
    ctx,
    { delayMs, createTask },
  ): Promise<Id<"_scheduled_functions">> => {
    if (createTask) {
      await ctx.db.insert("tasks", { author: "AI" });
    }

    return await ctx.scheduler.runAfter(delayMs, internal.scheduler.someAction);
  },
});

export const mutationSchedulingActionSchedulingAction = mutation({
  args: {},
  handler: async (ctx): Promise<Id<"_scheduled_functions">> => {
    return await ctx.scheduler.runAfter(
      0,
      api.scheduler.mutationSchedulingAction,
      { delayMs: 1000, createTask: true },
    );
  },
});

export const someAction = internalAction({ args: {}, handler: async () => {} });
