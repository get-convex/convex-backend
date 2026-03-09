import { getTransactionHeadroom } from "convex/server";
import { v } from "convex/values";
import { internalMutation } from "./_generated/server";
import { handleTaskDeletion } from "./lib/tasks";
import { internal } from "./_generated/api";

const MiB = 1 << 20;

export const clearTasks = internalMutation({
  args: {},
  handler: async (ctx, args) => {
    const tasks = ctx.db
      .query("tasks")
      .withIndex("by_status", (q) => q.eq("status", { archived: true }));

    for await (const task of tasks) {
      await handleTaskDeletion(ctx, task);
      await ctx.db.delete(task._id);
      const headroom = await getTransactionHeadroom();
      if (
        headroom.bytesRead.used > 4 * MiB ||
        headroom.bytesWritten.used > 2 * MiB ||
        headroom.databaseQueries.remaining < 500
      ) {
        // Run this mutation again and continue clearing tasks.
        await ctx.scheduler.runAfter(0, internal.clearTasks.clearTasks);
        break;
      }
    }
  },
});
