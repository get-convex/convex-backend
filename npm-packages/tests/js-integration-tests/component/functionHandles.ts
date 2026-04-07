import { createFunctionHandle } from "convex/server";
import { api, internal } from "./_generated/api";
import {
  action,
  internalAction,
  internalMutation,
  internalQuery,
  query,
} from "./_generated/server";
import { v } from "convex/values";

export const fromQuery = query(async () => {
  const handle: string = await createFunctionHandle(api.fileStorage.get);
  return handle;
});

export const fromAction = action(async () => {
  const handle: string = await createFunctionHandle(api.fileStorage.get);
  return handle;
});

export const q = internalQuery({
  args: {
    a: v.number(),
    b: v.number(),
  },
  returns: v.number(),
  handler: async (_ctx, { a, b }) => {
    return a + b;
  },
});

export const m = internalMutation({
  args: {
    a: v.number(),
    b: v.number(),
  },
  returns: v.number(),
  handler: async (_ctx, { a, b }) => {
    return a * b;
  },
});

export const a = internalAction({
  args: {
    a: v.number(),
    b: v.number(),
  },
  returns: v.number(),
  handler: async (_ctx, { a, b }) => {
    return a / b;
  },
});

export const getInternalHandle = query({
  args: {
    functionType: v.union(
      v.literal("query"),
      v.literal("mutation"),
      v.literal("action"),
    ),
  },
  returns: v.string(),
  handler: async (_ctx, { functionType }) => {
    let handle: string;
    switch (functionType) {
      case "query":
        handle = await createFunctionHandle(internal.functionHandles.q);
        break;
      case "mutation":
        handle = await createFunctionHandle(internal.functionHandles.m);
        break;
      case "action":
        handle = await createFunctionHandle(internal.functionHandles.a);
        break;
      default:
        throw new Error("Unexpected function type");
    }
    return handle;
  },
});
