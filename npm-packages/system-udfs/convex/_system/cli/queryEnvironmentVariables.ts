import { v } from "convex/values";
import { queryPrivateSystem } from "../secretSystemTables";

// This query returns a new result every time
// the given table's document change in any way.
export default queryPrivateSystem({
  args: {},
  handler: async ({ db }) => {
    return await db
      .query("_environment_variables")
      .withIndex("by_name")
      .order("asc")
      .collect();
  },
});

export const get = queryPrivateSystem({
  args: {
    name: v.string(),
  },
  handler: async ({ db }, { name }) => {
    return await db
      .query("_environment_variables")
      .withIndex("by_name", (q) => q.eq("name", name))
      .order("asc")
      .unique();
  },
});
