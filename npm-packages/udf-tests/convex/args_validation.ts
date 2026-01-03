import { v } from "convex/values";
import { mutation, query } from "./_generated/server";
import { Id } from "./_generated/dataModel";

export const stringArg = query({
  args: {
    arg: v.string(),
  },

  handler: (_, { arg }) => {
    return arg;
  },
});

export const returnRecord = mutation({
  args: {},
  handler: async (ctx) => {
    const boat = await ctx.db.insert("boats", { name: "boat" });
    const boatVote = await ctx.db.insert("boatVotes", { boat });
    const result: Record<Id<"boats"> | Id<"boatVotes">, number> = {};
    result[boat] = 1;
    result[boatVote] = 2;
    return result;
  },
});

export const recordArg = query({
  args: {
    // This is a silly record, but is the easiest way to test a subtype of string
    // as a key
    arg: v.record(v.union(v.id("boats"), v.id("boatVotes")), v.number()),
  },

  handler: (_, { arg }) => {
    return arg;
  },
});

// Test for issue #212: BigInt literals in validators should work with function-spec
export const bigintLiteralArgs = query({
  args: {
    i: v.literal(BigInt(1)),
  },
  returns: v.literal(BigInt(1)),
  handler: (ctx, args) => {
    return args.i;
  },
});

// Additional test with multiple BigInt literals
export const multipleBigintLiterals = query({
  args: {
    small: v.literal(BigInt(1)),
    large: v.literal(BigInt(9223372036854775807)), // max i64
    negative: v.literal(BigInt(-42)),
  },
  handler: (_, args) => {
    return { ...args };
  },
});
