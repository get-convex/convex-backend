import { api } from "./_generated/api";
import { Id } from "./_generated/dataModel";
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import { query, mutation } from "./_generated/server";
import { components } from "./_generated/api";
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

export const scheduleInParent = mutation({
  args: { message: v.string() },
  handler: async (ctx, { message }): Promise<Id<"_scheduled_functions">> => {
    return await ctx.scheduler.runAfter(0, api.scheduler.sendMessage, {
      message,
    });
  },
});

export const scheduleWithinComponent = mutation({
  args: { message: v.string() },
  handler: async (ctx, { message }) => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    return await ctx.runMutation(
      components.component.scheduler.scheduleWithinComponent,
      { message },
    );
  },
});

export const scheduleChildFromParent = mutation({
  args: { message: v.string() },
  handler: async (ctx, { message }) => {
    return await ctx.scheduler.runAfter(
      0,
      components.component.scheduler.sendMessage,
      { message },
    );
  },
});

export const statusInParent = query({
  args: { id: v.string() },
  handler: async (ctx, { id }) => {
    const f = await ctx.db.system.get(id as Id<"_scheduled_functions">);
    return f?.state.kind;
  },
});

export const statusInComponent = query({
  args: { id: v.string() },
  handler: async (ctx, { id }) => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    return await ctx.runQuery(components.component.scheduler.status, { id });
  },
});

export const listAllMessagesInParent = query({
  handler: async (ctx) => {
    return await ctx.db.query("messages").collect();
  },
});

export const listAllMessagesInComponent = query({
  handler: async (ctx) => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    return await ctx.runQuery(components.component.scheduler.listAllMessages);
  },
});

export const cancelSelf = mutation({
  handler: async ({ db, scheduler }) => {
    const allJobs = await db.system.query("_scheduled_functions").collect();
    for (const job of allJobs) {
      if (job.state.kind === "pending" || job.state.kind === "inProgress") {
        await scheduler.cancel(job._id);
      }
    }
  },
});

export const scheduleSelfCanceling = mutation({
  handler: async ({ scheduler }): Promise<Id<"_scheduled_functions">> => {
    return await scheduler.runAfter(0, api.scheduler.cancelSelf, {});
  },
});
