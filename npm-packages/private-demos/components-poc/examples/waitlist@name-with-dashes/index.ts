import { v } from "convex/values";
import { query, mutation, action } from "./_generated/server";
import { api } from "./_generated/api";

export const repeatMessage = action({
  args: { message: v.string(), n: v.number() },
  returns: v.string(),
  handler: async (_ctx, args: any) => {
    const output = args.message.repeat(args.n);
    console.log("repeating", args.message, args.n, "times");
    return output;
  },
});

export const sayGoodbyeFromQuery = query({
  args: {},
  returns: v.string(),
  handler: async (ctx) => {
    const results = await ctx.db
      .query("roomMember")
      .withIndex("by_active", (q) => q.eq("active", false))
      .collect();
    console.log("results", results);
    console.log("saying goodbye");
    return `You say goodbye`;
  },
});

export const sayHelloFromMutation = mutation({
  args: {},
  returns: v.string(),
  handler: async (ctx) => {
    await ctx.db.insert("roomMember", { identifier: "test", active: false });
    console.log("saying hello");
    return "and I say hello, hello, hello";
  },
});

export const scheduleMessage = mutation({
  args: {},
  handler: async (ctx) => {
    console.log("scheduling message");
    await ctx.db.insert("messages", { text: "scheduled" });
    await ctx.scheduler.runAfter(1000, api.index.scheduleMessage, {});
    return "scheduled";
  },
});

export const scheduleSend = mutation({
  args: {},
  handler: async (ctx) => {
    console.log("scheduling message");
    await ctx.scheduler.runAfter(30 * 1000, api.index.sendMessage, {});
    return "scheduled";
  },
});

export const sendMessage = mutation({
  args: {},
  handler: async (ctx) => {
    console.log("sending message");
    await ctx.db.insert("messages", { text: "sent" });
    return "sent";
  },
});

export const getMessageCount = query({
  args: {},
  returns: v.number(),
  handler: async (ctx) => {
    return (await ctx.db.query("messages").collect()).length;
  },
});

export const storeInFile = action({
  args: { message: v.string() },
  returns: v.id("_storage"),
  handler: async (ctx, args) => {
    const blob = new Blob([args.message], { type: "text/plain" });
    return await ctx.storage.store(blob);
  },
});

export const readFromFile = action({
  args: { id: v.id("_storage") },
  returns: v.string(),
  handler: async (ctx, args) => {
    const blob = await ctx.storage.get(args.id);
    return await blob!.text();
  },
});

export const listFiles = query(async (ctx) => {
  return await ctx.db.system.query("_storage").collect();
});

export const fileUploadUrl = mutation({
  args: {},
  returns: v.string(),
  handler: async (ctx) => {
    return ctx.storage.generateUploadUrl();
  },
});

export const fileDownloadUrl = query({
  args: { id: v.id("_storage") },
  returns: v.string(),
  handler: async (ctx, args) => {
    const url = await ctx.storage.getUrl(args.id);
    return url!;
  },
});

export const writeThenFail = mutation({
  args: { text: v.string() },
  handler: async (ctx, { text }) => {
    await ctx.db.insert("messages", { text });
    throw new Error("failed");
  },
});

export const writeSuccessfully = mutation({
  args: { text: v.string() },
  handler: async (ctx, { text }) => {
    await ctx.db.insert("messages", { text });
    return "succeeded";
  },
});

export const latestWrite = query({
  args: {},
  returns: v.string(),
  handler: async (ctx) => {
    const message = await ctx.db.query("messages").order("desc").first();
    return message!.text;
  },
});
