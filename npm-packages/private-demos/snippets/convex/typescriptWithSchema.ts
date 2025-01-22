import { query } from "./_generated/server";

export const list = query({
  args: {},
  // The inferred return type of `handler` is now `Promise<Doc<"messages">[]>`
  handler: (ctx) => {
    return ctx.db.query("messages").collect();
  },
});
