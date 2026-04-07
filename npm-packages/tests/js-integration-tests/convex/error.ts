import { query, mutation } from "./_generated/server";

export default query(async () => {
  throw new Error("oopsie");
});

declare const Convex: { syscall: (op: string, jsonArgs: string) => string };

export const occ = mutation({
  args: {},
  handler: async (ctx) => {
    if ((await ctx.db.query("messages").first()) !== null) {
      return;
    }
    Convex.syscall("throwOcc", "{}");
  },
});

export const overloaded = mutation({
  args: {},
  handler: async (ctx) => {
    if ((await ctx.db.query("messages").first()) !== null) {
      return;
    }
    Convex.syscall("throwOverloaded", "{}");
  },
});
