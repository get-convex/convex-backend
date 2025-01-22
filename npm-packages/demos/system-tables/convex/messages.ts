import { v } from "convex/values";
import { query, mutation } from "./_generated/server";
import { api } from "./_generated/api";
import { Id } from "./_generated/dataModel";

export const list = query({
  args: {},
  handler: async (ctx) => {
    const messages = await ctx.db.query("messages").collect();
    return Promise.all(
      messages.map(async (message) => ({
        ...message,
        // If the message is an "image" its `body` is a `StorageId`
        ...(message.format === "image"
          ? { url: await ctx.storage.getUrl(message.body) }
          : {}),
      })),
    );
  },
});

export const generateUploadUrl = mutation(async (ctx) => {
  return await ctx.storage.generateUploadUrl();
});

export const sendImage = mutation({
  args: { storageId: v.id("_storage"), author: v.string() },
  handler: async (ctx, args) => {
    await ctx.db.insert("messages", {
      body: args.storageId,
      author: args.author,
      format: "image",
    });
    await ctx.db.insert("file_uploads", {
      author: args.author,
      upload_id: args.storageId,
    });
  },
});

export const sendMessage = mutation({
  args: { body: v.string(), author: v.string() },
  handler: async (ctx, args) => {
    const { body, author } = args;
    await ctx.db.insert("messages", { body, author, format: "text" });
  },
});

export const scheduleMessage = mutation({
  args: { delay: v.float64(), body: v.string(), author: v.string() },
  handler: async (ctx, args) => {
    const job_id: Id<"_scheduled_functions"> = await ctx.scheduler.runAfter(
      args.delay * 1000,
      api.messages.sendMessage,
      {
        body: args.body,
        author: args.author,
      },
    );
    return job_id;
  },
});
