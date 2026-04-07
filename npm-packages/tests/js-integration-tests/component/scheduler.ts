import { api } from "./_generated/api";
import { Id } from "./_generated/dataModel";
import { query, mutation } from "./_generated/server";
import { v } from "convex/values";

export const sendMessage = mutation({
  args: { message: v.string() },
  handler: async (ctx, { message }) => {
    return await ctx.db.insert("messages", {
      channel: "general",
      text: message,
    });
  },
});

export const scheduleWithinComponent = mutation({
  args: { message: v.string() },
  returns: v.id("_scheduled_functions"),
  handler: async (ctx, { message }): Promise<Id<"_scheduled_functions">> => {
    return await ctx.scheduler.runAfter(0, api.scheduler.sendMessage, {
      message,
    });
  },
});

export const status = query({
  args: { id: v.string() },
  handler: async (ctx, { id }) => {
    const f = await ctx.db.system.get(id as Id<"_scheduled_functions">);
    return f?.state.kind;
  },
});

export const listAllMessages = query({
  handler: async (ctx) => {
    return await ctx.db.query("messages").collect();
  },
});
