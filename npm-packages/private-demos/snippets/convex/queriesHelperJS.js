import { v } from "convex/values";
import { query } from "./_generated/server";

export const getTaskAndAuthor = query({
  args: { id: v.id("tasks") },
  handler: async (ctx, args) => {
    const task = await ctx.db.get(args.id);
    if (task === null) {
      return null;
    }
    return { task, author: await getUserName(ctx, task.authorId ?? null) };
  },
});

async function getUserName(ctx, userId) {
  if (userId === null) {
    return null;
  }
  return (await ctx.db.get(userId))?.name;
}
