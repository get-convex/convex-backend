import { v } from "convex/values";
import { query } from "./_generated/server";

export const stringArg = query({
  args: {
    arg: v.string(),
  },

  handler: (_, { arg }) => {
    return arg;
  },
});

export const recordArg = query({
  args: {
    // This is a silly record, but is the easiest way to test a subtype of string
    // as a key
    arg: v.record(
      v.union(v.literal("foo"), v.literal("bar"), v.literal("baz")),
      v.optional(v.number()),
    ),
  },

  handler: (_, { arg }) => {
    return arg;
  },
});
