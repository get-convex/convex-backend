import { v } from "convex/values";
import { action, mutation, query } from "./_generated/server";

export const queryWithStringArg = query({
  args: {
    name: v.string(),
  },
  handler: (_, { name }) => {
    return `Hello, ${name}!`;
  },
});

export const queryWithNumberArg = query({
  args: {
    count: v.number(),
  },
  handler: (_, { count }) => {
    return count * 2;
  },
});

export const queryWithObjectArg = query({
  args: {
    user: v.object({
      name: v.string(),
      age: v.number(),
    }),
  },
  handler: (_, { user }) => {
    return user;
  },
});

export const mutationWithRequiredArgs = mutation({
  args: {
    channel: v.string(),
    text: v.string(),
  },
  handler: async (ctx, { channel, text }) => {
    return await ctx.db.insert("messages", { channel, text });
  },
});

export const actionWithValidation = action({
  args: {
    email: v.string(),
    count: v.number(),
  },
  handler: async (_, { email, count }) => {
    return { email, count };
  },
});

export const queryWithMultipleArgs = query({
  args: {
    required: v.string(),
    optional: v.optional(v.string()),
    number: v.number(),
  },
  handler: (_, { required, optional, number }) => {
    return { required, optional, number };
  },
});

export const queryWithUnion = query({
  args: {
    value: v.union(v.string(), v.number()),
  },
  handler: (_, { value }) => {
    return value;
  },
});

export const queryWithArray = query({
  args: {
    items: v.array(v.string()),
  },
  handler: (_, { items }) => {
    return items.length;
  },
});
