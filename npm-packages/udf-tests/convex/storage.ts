import { query, action } from "./_generated/server";
import { v } from "convex/values";

export const storeFile = action({
  args: { data: v.bytes() },
  handler: async (ctx, { data }) => {
    return ctx.storage.store(new Blob([data]));
  },
});

export const getFile = action({
  args: { id: v.id("_storage") },
  handler: async (ctx, { id }) => {
    const blob = await ctx.storage.get(id);
    return await blob!.arrayBuffer();
  },
});

export const getFileUrl = query({
  args: { id: v.id("_storage") },
  handler: async (ctx, { id }) => {
    return ctx.storage.getUrl(id);
  },
});

export const getFileUrlFromAction = action({
  args: { id: v.id("_storage") },
  handler: async (ctx, { id }) => {
    return ctx.storage.getUrl(id);
  },
});

export const getFileUrls = query({
  args: { ids: v.array(v.id("_storage")) },
  handler: async (ctx, { ids }) => {
    return Promise.all(ids.map((id) => ctx.storage.getUrl(id)));
  },
});
