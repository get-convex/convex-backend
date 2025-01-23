import { v } from "convex/values";
import { mutation, query } from "./_generated/server";

export const increment = mutation({
  args: { increment: v.number() },
  handler: () => {
    // empty
  },
});

export const get = query({
  handler: () => {
    return 1;
  },
});
