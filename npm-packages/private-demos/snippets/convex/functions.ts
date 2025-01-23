import { v } from "convex/values";
import { query } from "./_generated/server";

export const read = query({
  args: { param: v.string() },
  handler: async (ctx, args) => {},
});
