import { action } from "./_generated/server";
import { v } from "convex/values";

export const doSomething = action({
  args: { a: v.number(), b: v.number() },
  handler: (ctx, args) => {
    // do something with `ctx`
  },
});
