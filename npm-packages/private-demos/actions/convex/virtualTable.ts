import { mutation, query } from "./_generated/server";
import { Id } from "./_generated/dataModel";
import { api } from "./_generated/api";

// List all scheduled jobs
export const listJobs = query({
  handler: async ({ db }) => {
    return await db.system
      .query("_scheduled_functions")
      .withIndex("by_creation_time", (q) => q.gte("_creationTime", 0))
      .collect();
  },
});

// List all files from storage
export const listFiles = query({
  handler: async ({ db }) => {
    return await db.system.query("_storage").collect();
  },
});

// List all messages
export const listMessages = query({
  handler: async ({ db }) => {
    return await db.query("messages").collect();
  },
});

// Get one job
export const getJob = query({
  handler: async ({ db }, { id }: { id: Id<"_scheduled_functions"> }) => {
    return await db.system.get(id);
  },
});

// Get one file metadata
export const getFile = query({
  handler: async ({ db }, { id }: { id: Id<"_storage"> }) => {
    return await db.system.get(id);
  },
});

// Get one message
export const getMessage = query({
  handler: async ({ db }, { id }: { id: Id<"messages"> }) => {
    return await db.get(id);
  },
});

export const scheduleJob = mutation({
  handler: async (ctx): Promise<Id<"_scheduled_functions">> => {
    return await ctx.scheduler.runAfter(0, api.virtualTable.placeholder);
  },
});

export const placeholder = mutation({
  handler: async () => {},
});

// Can't use db.system.query for a user table
export const runtimeError1 = query({
  handler: async ({ db }) => {
    return await db.system
      .query("messages" as "_scheduled_functions")
      .collect();
  },
});

// Can't use db.query for a system table
export const runtimeError2 = query({
  handler: async ({ db }) => {
    return await db.query("_scheduled_functions" as "messages").collect();
  },
});

// Can't use db.system.get for a user-table id
export const runtimeError3 = query({
  handler: async ({ db }, { id }: { id: Id<"messages"> }) => {
    return await db.system.get(id as unknown as Id<"_scheduled_functions">);
  },
});

// Can't use db.get for a system-table id
export const runtimeError4 = query({
  handler: async ({ db }, { id }: { id: Id<"_scheduled_functions"> }) => {
    return await db.get(id as unknown as Id<"messages">);
  },
});

// Can't perform db.insert on system tables
export const runtimeError5 = mutation({
  handler: async ({ db }) => {
    const fakeDoc = { name: "anjan" };
    return await db.insert(
      "_scheduled_functions" as "messages",
      fakeDoc as any,
    );
  },
});

// Can't perform db.patch on system tables
export const runtimeError6 = mutation({
  handler: async ({ db }, { id }: { id: Id<"_scheduled_functions"> }) => {
    const fakeDoc = { name: "anjan" };
    return await db.patch(id as unknown as Id<"messages">, fakeDoc as any);
  },
});

// Can't perform db.replace on system tables
export const runtimeError7 = mutation({
  handler: async ({ db }, { id }: { id: Id<"_scheduled_functions"> }) => {
    const fakeDoc = { name: "anjan" };
    return await db.replace(id as unknown as Id<"messages">, fakeDoc as any);
  },
});

// Can't perform db.delete on system tables
export const runtimeError8 = mutation({
  handler: async ({ db }, { id }: { id: Id<"_scheduled_functions"> }) => {
    return await db.delete(id as unknown as Id<"messages">);
  },
});
