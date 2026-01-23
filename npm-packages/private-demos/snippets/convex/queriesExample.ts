import { query } from "./_generated/server";
import { v } from "convex/values";

// Return the last 100 tasks in a given task list.
export const getTaskList = query({
  args: { taskListId: v.id("taskLists") },
  handler: async (ctx, args) => {
    const tasks = await ctx.db
      .query("tasks")
      .withIndex("by_task_list_id", (q) => q.eq("taskListId", args.taskListId))
      .order("desc")
      .take(100);
    return tasks;
  },
});
