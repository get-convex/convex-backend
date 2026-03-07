import { query } from "./_generated/server";
import { v } from "convex/values";

export const averageGrade = query({
  args: { studentId: v.string() },
  handler: async (ctx, args) => {
    const grades = await ctx.db
      .query("grades")
      .withIndex("by_studentId", (q) => q.eq("studentId", args.studentId))
      .collect();

    // highlight-start
    const sum = grades.reduce((soFar, { grade }) => soFar + grade, 0);
    return sum / grades.length;
    // highlight-end
  },
});
