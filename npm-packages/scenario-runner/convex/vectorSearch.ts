import { v } from "convex/values";
import { action } from "./_generated/server";
import { OPENCLAURD_TABLE } from "../types";

export default action({
  args: {
    vector: v.array(v.float64()),
    limit: v.float64(),
    users: v.array(v.string()),
  },
  handler: async (ctx, args) => {
    return await ctx.vectorSearch(OPENCLAURD_TABLE, "embedding", {
      vector: args.vector,
      limit: args.limit,
      filter: (q) => q.or(...args.users.map((user) => q.eq("user", user))),
    });
  },
});
