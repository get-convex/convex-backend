import { v } from "convex/values";
import { api } from "./_generated/api";
import { Id } from "./_generated/dataModel";
import { query, mutation } from "./_generated/server";
import { maskSystemWriter } from "./secretSystemTables";

// Query every system table
export const queryAll = query({
  args: {},
  handler: async ({ db }) => {
    await db.system.query("_scheduled_functions").collect();
    await db.system.query("_storage").collect();
  },
});

// List all scheduled jobs
export const listJobs = query({
  args: {},
  handler: async ({ db }) => {
    return await db.system.query("_scheduled_functions").collect();
  },
});

// List all messages
export const listMessages = query({
  args: {},
  handler: async ({ db }) => {
    return await db.query("messages").collect();
  },
});

// Get one job
export const getJob = query({
  args: { id: v.id("_scheduled_functions") },
  handler: async ({ db }, args) => {
    return await db.system.get(args.id);
  },
});

// Get one message
export const getMessage = query({
  args: { id: v.id("messages") },
  handler: async ({ db }, args) => {
    return await db.get(args.id);
  },
});

export const scheduleJob = mutation({
  args: {},
  handler: async (ctx) => {
    await ctx.scheduler.runAfter(0, api.systemTables.placeholder);
  },
});

export const placeholder = mutation({
  args: {},
  handler: async () => {},
});

// Can't use db.system.query for a user table
export const badSystemQuery = query({
  args: {},
  handler: async ({ db }) => {
    return await db.system
      .query("messages" as "_scheduled_functions")
      .collect();
  },
});

// Can't use db.query for a system table
export const badUserQuery = query({
  args: {},
  handler: async ({ db }) => {
    return await db.query("_scheduled_functions" as "messages").collect();
  },
});

// Can't use db.system.get for a user-table id
export const badSystemGet = query({
  args: { id: v.id("messages") },
  handler: async ({ db }, args) => {
    return await db.system.get(
      args.id as unknown as Id<"_scheduled_functions">,
    );
  },
});

// Can't use db.get for a system-table id
export const badUserGet = query({
  args: { id: v.id("_scheduled_functions") },
  handler: async ({ db }, args) => {
    return await db.get(args.id as unknown as Id<"messages">);
  },
});

// Can't perform db.insert on system tables
export const badSystemInsert = mutation({
  args: {},
  handler: async ({ db }) => {
    const fakeDoc = { name: "anjan" };
    return await db.insert(
      "_scheduled_functions" as "messages",
      fakeDoc as any,
    );
  },
});

// Can't perform db.patch on system tables
export const badSystemPatch = mutation({
  args: { id: v.id("_scheduled_functions") },
  handler: async ({ db }, args) => {
    const fakeDoc = { name: "anjan" };
    return await db.patch(args.id as unknown as Id<"messages">, fakeDoc as any);
  },
});

// Can't perform db.replace on system tables
export const badSystemReplace = mutation({
  args: { id: v.id("_scheduled_functions") },
  handler: async ({ db }, args) => {
    const fakeDoc = { name: "anjan" };
    return await db.replace(
      args.id as unknown as Id<"messages">,
      fakeDoc as any,
    );
  },
});

// Can't perform db.delete on system tables
export const badSystemDelete = mutation({
  args: { id: v.id("_scheduled_functions") },
  handler: async ({ db }, args) => {
    return await db.delete(args.id as unknown as Id<"messages">);
  },
});

// db.system object doesn't have a .insert() method, this throws at JS runtime
export const systemInsertJSError = mutation({
  args: {},
  handler: async ({ db }) => {
    const fakeDoc = { name: "anjan" };
    return await maskSystemWriter(db).insert(
      "_scheduled_functions" as "messages",
      fakeDoc as any,
    );
  },
});

// db.system object doesn't have a .patch() method, this throws at JS runtime
export const systemPatchJSError = mutation({
  args: { id: v.id("_scheduled_functions") },
  handler: async ({ db }, args) => {
    const fakeDoc = { name: "anjan" };
    return await maskSystemWriter(db).patch(
      args.id as unknown as Id<"messages">,
      fakeDoc as any,
    );
  },
});

// db.system object doesn't have a .replace() method, this throws at JS runtime
export const systemReplaceJSError = mutation({
  args: { id: v.id("_scheduled_functions") },
  handler: async ({ db }, args) => {
    const fakeDoc = { name: "anjan" };
    return await maskSystemWriter(db).replace(
      args.id as unknown as Id<"messages">,
      fakeDoc as any,
    );
  },
});

// db.system object doesn't have a .delete() method, this throws at JS runtime
export const systemDeleteJSError = mutation({
  args: { id: v.id("_scheduled_functions") },
  handler: async ({ db }, args) => {
    return await maskSystemWriter(db).delete(
      args.id as unknown as Id<"messages">,
    );
  },
});

// tests that virtual ids can exist on schemas
export const setForeignVirtualId = mutation({
  args: {},
  handler: async (ctx) => {
    const jobId: Id<"_scheduled_functions"> = await ctx.scheduler.runAfter(
      0,
      api.systemTables.placeholder,
    );
    return await ctx.db.insert("virtualForeignKeys", {
      foreignKeyField: jobId,
    });
  },
});
