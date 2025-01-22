import { Id } from "./_generated/dataModel";
import { query, QueryCtx } from "./_generated/server";
import { v } from "convex/values";

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

async function getUserName(ctx: QueryCtx, userId: Id<"users"> | null) {
  if (userId === null) {
    return null;
  }
  return (await ctx.db.get(userId))?.name;
}
