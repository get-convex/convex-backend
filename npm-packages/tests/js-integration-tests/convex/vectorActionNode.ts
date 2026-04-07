"use node";

import { v } from "convex/values";
import { action } from "./_generated/server";
import { vectorSearchHandler } from "./vectorActionV8";

export const vectorSearch = action({
  args: { embedding: v.array(v.float64()), cuisine: v.string() },
  handler: async (ctx, args) => vectorSearchHandler(ctx, args),
});
