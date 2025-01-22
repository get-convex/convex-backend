// @snippet start action
import { action, internalQuery } from "./_generated/server";
import { internal } from "./_generated/api";
import { v } from "convex/values";

export const doSomething = action({
  args: { a: v.number() },
  handler: async (ctx, args) => {
    const data = await ctx.runQuery(internal.myFunctions.readData, {
      a: args.a,
    });
    // do something with `data`
  },
});

export const readData = internalQuery({
  args: { a: v.number() },
  handler: async (ctx, args) => {
    // read from `ctx.db` here
  },
});
// @snippet end action

// Used by client or internal call examples
import { mutation, query } from "./_generated/server";
import { paginationOptsValidator } from "convex/server";
import { internalAction } from "./_generated/server";

export const sum = query({
  args: { a: v.number(), b: v.number() },
  handler: (_, args) => {
    return args.a + args.b;
  },
});

export const getSomething = query({
  args: {},
  handler: () => {
    return null;
  },
});

export const mutateSomething = mutation({
  args: { a: v.number(), b: v.number() },
  handler: (_, args): void => {
    // do something with `a` and `b`
  },
});

export const getSomethingPaginated = query({
  args: {
    paginationOpts: paginationOptsValidator,
  },
  handler: (ctx, args) => {
    return ctx.db.query("foods").paginate(args.paginationOpts);
  },
});

export const actionThatCallsAPI = internalAction({
  args: { taskId: v.id("tasks"), text: v.string() },
  handler: (_, args): void => {
    // do something with `taskId` and `text`, like call an API
    // then run another mutation to store the result
  },
});
