// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import { query, mutation, action } from "./_generated/server";
import { components } from "./_generated/api";
import { v } from "convex/values";

export const generateUploadUrl = mutation({
  args: {},
  handler: async (ctx) => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    return ctx.runMutation(components.component.fileStorage.generateUploadUrl);
  },
});

export const getUrl = query({
  args: { storageId: v.string() },
  handler: async (ctx, { storageId }) => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    return ctx.runQuery(components.component.fileStorage.getUrl, { storageId });
  },
});

export const list = query({
  args: {},
  handler: async (ctx) => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    return ctx.runQuery(components.component.fileStorage.list);
  },
});

export const get = query({
  args: { id: v.string() },
  handler: async (ctx, { id }) => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    return ctx.runQuery(components.component.fileStorage.get, { id });
  },
});

export const storeFile = action({
  args: { data: v.string() },
  handler: async (ctx, { data }) => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    return ctx.runAction(components.component.fileStorage.storeFile, { data });
  },
});

export const getFile = action({
  args: { storageId: v.string() },
  handler: async (ctx, { storageId }) => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    return ctx.runAction(components.component.fileStorage.getFile, {
      storageId,
    });
  },
});
