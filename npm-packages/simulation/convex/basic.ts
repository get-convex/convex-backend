import { mutation, query } from "./_generated/server";
import { v } from "convex/values";

export const count = query({
  args: {},
  handler: async ({ db }) => {
    let count = 0;
    for await (const _ of db.query("objects")) {
      count++;
    }
    return count;
  },
});

export const insertObject = mutation({
  args: v.any(),
  handler: async ({ db }, obj) => {
    const id = await db.insert("objects", obj);
    return await db.get(id);
  },
});
