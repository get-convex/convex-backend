import { mutation, query, action } from "./_generated/server";
import { v } from "convex/values";

export const intQuery = query(async () => {
  return 1n;
});

export const intMutation = mutation(async () => {
  return 1n;
});

export const intAction = action(async () => {
  return 1n;
});

export const insertObject = mutation({
  args: { obj: v.any() },
  handler: async (ctx, { obj }) => {
    return ctx.db.insert("test", obj);
  },
});

export const getObject = query({
  args: { id: v.id("test") },
  handler: async (ctx, { id }) => {
    return ctx.db.get(id);
  },
});
