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

export const objectArgStrip = query({
  args: v.object({ arg: v.string() }, { unknownKeys: "strip" }),

  handler: (_, args) => {
    return args;
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
