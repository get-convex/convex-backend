"use node";

import { v } from "convex/values";
import { action } from "./_generated/server";

export const danglingFetch = action({
  args: { url: v.string() },
  handler: async (_, { url }) => {
    console.log("hitting url:", url);
    void fetch(url);

    return "dangling fetch result";
  },
});
