"use node";
import { v } from "convex/values";
import { api } from "./_generated/api";
import { action } from "./_generated/server";

export default action({
  args: {
    author: v.string(),
  },
  handler: async ({ runMutation }, { author }: { author: string }) => {
    await runMutation(api.sendMessage.default, {
      format: "text",
      body: "toe",
      author,
    });
  },
});
