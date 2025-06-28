"use node";
import { v } from "convex/values";
import { api } from "./_generated/api";
import { action } from "./_generated/server";

export default action({
  args: {
    author: v.string(),
  },
  handler: async (ctx, { author }) => {
    console.log("running toe mutation");
    await ctx.runMutation(api.sendMessage.default, {
      format: "text",
      body: "toe",
      author,
    });
  },
});
