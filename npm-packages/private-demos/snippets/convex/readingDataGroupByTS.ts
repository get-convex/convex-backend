import { query } from "./_generated/server";
import { v } from "convex/values";

export const numGradesBySubjectForStudent = query({
  args: { studentId: v.string() },
  handler: async (ctx, args) => {
    const grades = await ctx.db
      .query("grades")
      .withIndex("by_studentId", (q) => q.eq("studentId", args.studentId))
      .collect();

    // highlight-start
    const counts: Record<string, number> = {};
    for (const { subject } of grades) {
      counts[subject] = (counts[subject] ?? 0) + 1;
    }
    return counts;
    // highlight-end
  },
});
