import { v } from "convex/values";
import { api } from "./_generated/api";
import { Id } from "./_generated/dataModel";
import { mutation } from "./_generated/server";

export default mutation({
  args: {
    author: v.string(),
  },
  handler: async (
    { db, scheduler },
    { author },
  ): Promise<Id<"_scheduled_functions">> => {
    const message = {
      format: "text" as const,
      body: "tic",
      author,
    };
    await db.insert("messages", message);
    return await scheduler.runAfter(1000, api.tac.default, { author });
  },
});
