import { query, mutation, action } from "./_generated/server";
import { v } from "convex/values";

export const generateUploadUrl = mutation(async ({ storage }) => {
  return await storage.generateUploadUrl();
});

export const getUrl = query({
  args: { storageId: v.id("_storage") },
  handler: async ({ storage }, { storageId }) => {
    return await storage.getUrl(storageId);
  },
});

export const storeFile = action({
  args: { data: v.string() },
  handler: async ({ storage }, { data }) => {
    return await storage.store(new Blob([data], { type: "text/plain" }));
  },
});

export const getFile = action({
  args: { storageId: v.id("_storage") },
  handler: async ({ storage }, { storageId }) => {
    const blob = await storage.get(storageId);
    return await blob!.text();
  },
});

export const deleteById = mutation({
  args: { storageId: v.id("_storage") },
  handler: async ({ storage }, { storageId }) => {
    return await storage.delete(storageId);
  },
});

export const list = query({
  handler: async ({ db }) => {
    return await db.system.query("_storage").collect();
  },
});

export const get = query({
  args: { id: v.id("_storage") },
  handler: async ({ db }, { id }) => {
    return await db.system.get(id);
  },
});
