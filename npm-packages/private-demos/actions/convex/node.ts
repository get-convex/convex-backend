"use node";

import { v } from "convex/values";
import { action } from "./_generated/server";

export const node = action({
  args: {},
  returns: v.number(),
  handler: async () => {
    return "dangling fetch result" as any;
  },
});
