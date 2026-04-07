import { v } from "convex/values";
import { action, mutation, query } from "./_generated/server";

export const extraOutputFields = query({
  args: {},
  returns: v.object({ a: v.number() }),
  handler: () => {
    return { a: 1, b: 2 } as any;
  },
});

export const stringOutputReturnsNumberQuery = query({
  args: {},
  returns: v.string(),
  handler: () => {
    return 1 as unknown as Promise<string>;
  },
});

export const stringOutputReturnsNumberMutation = mutation({
  args: {},
  returns: v.string(),
  handler: () => {
    return 1 as unknown as Promise<string>;
  },
});

export const stringOutputReturnsNumberAction = action({
  args: {},
  returns: v.string(),
  handler: () => {
    return 1 as unknown as Promise<string>;
  },
});

export const stringOutputReturnsStringQuery = query({
  args: {},
  returns: v.string(),
  handler: () => {
    return "hello";
  },
});

export const stringOutputReturnsStringMutation = mutation({
  args: {},
  returns: v.string(),
  handler: () => {
    return "hello";
  },
});

export const stringOutputReturnsStringAction = action({
  args: {},
  returns: v.string(),
  handler: () => {
    return "hello";
  },
});
