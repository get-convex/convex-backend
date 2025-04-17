"use node";
import { v } from "convex/values";
import { action } from "./_generated/server";
import fetch from "node-fetch";

export const fetchUrl = action({
  args: { url: v.string() },
  handler: async (_, { url }): Promise<string> => {
    const result = await fetch(url);
    return await result.text();
  },
});
