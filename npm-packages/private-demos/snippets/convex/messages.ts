// @snippet start scheduling-runAfter
import { mutation, internalMutation } from "./_generated/server";
import { internal } from "./_generated/api";
import { v } from "convex/values";

export const sendExpiringMessage = mutation({
  args: { body: v.string(), author: v.string() },
  handler: async (ctx, args) => {
    const { body, author } = args;
    const id = await ctx.db.insert("messages", { body, author });
    await ctx.scheduler.runAfter(5000, internal.messages.destruct, {
      messageId: id,
    });
  },
});

export const destruct = internalMutation({
  args: {
    messageId: v.id("messages"),
  },
  handler: async (ctx, args) => {
    await ctx.db.delete(args.messageId);
  },
});
// @snippet end scheduling-runAfter

// @snippet start scheduling-status
export const listScheduledMessages = query({
  args: {},
  handler: async (ctx, args) => {
    return await ctx.db.system.query("_scheduled_functions").collect();
  },
});

export const getScheduledMessage = query({
  args: {
    id: v.id("_scheduled_functions"),
  },
  handler: async (ctx, args) => {
    return await ctx.db.system.get(args.id);
  },
});
// @snippet end scheduling-status

// @snippet start scheduling-cancel
export const cancelMessage = mutation({
  args: {
    id: v.id("_scheduled_functions"),
  },
  handler: async (ctx, args) => {
    await ctx.scheduler.cancel(args.id);
  },
});
// @snippet end scheduling-cancel

// Don't use this in examples, it's here just for typechecking

export const clearAll = internalMutation({
  args: {},
  handler: async () => {
    // empty
  },
});

import { query } from "./_generated/server";
export const list = query({
  args: {
    channel: v.string(),
  },
  handler: async (ctx) => {
    return await ctx.db.query("messages").collect();
  },
});

export const send = mutation({
  args: { body: v.string(), channel: v.id("channels") },
  handler: async (ctx, args) => {
    const { body, channel } = args;
    const id = await ctx.db.insert("messages", { body, channel });
    return id;
  },
});

// just here for typechecking
export const sendAnon = mutation({
  args: { body: v.string() },
  handler: async () => {},
});
// just here for typechecking
export const listAll = query({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.query("messages").collect();
  },
});

export const sendOne = internalMutation({
  args: { body: v.string(), author: v.string() },
  handler: async () => {
    // empty
  },
});

export const like = mutation({
  args: { liker: v.string(), messageId: v.id("messages") },
  handler: async () => {
    // empty
  },
});

export const getForCurrentUser = query({
  args: {},
  handler: async () => {
    // empty
  },
});
