import { internalMutation } from "./_generated/server";
import { v } from "convex/values";

export const writeData = internalMutation({
  args: { a: v.number() },
  handler: (_, _args) => {
    // empty
  },
});
