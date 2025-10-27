import { query } from "./_generated/server";

export const listTasks = query({
  args: {},
  handler: async (ctx) => {
    const tasks = await ctx.db.query("tasks").collect();
    // do something with `tasks`
  },
});
