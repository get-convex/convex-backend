import { RequestMetadata } from "convex/server";
import { api } from "./_generated/api";
import { mutation, action } from "./_generated/server";

// Direct request metadata access
export const fromMutation = mutation({
  args: {},
  handler: async (ctx): Promise<RequestMetadata> => {
    return await ctx.meta.getRequestMetadata();
  },
});

export const fromAction = action({
  args: {},
  handler: async (ctx): Promise<RequestMetadata> => {
    return await ctx.meta.getRequestMetadata();
  },
});

// V8 action calling nested mutation
export const fromActionCallingMutation = action({
  args: {},
  handler: async (ctx): Promise<RequestMetadata> => {
    return await ctx.runMutation(api.requestMetadata.fromMutation, {});
  },
});

// V8 action calling nested V8 action
export const fromActionCallingAction = action({
  args: {},
  handler: async (ctx): Promise<RequestMetadata> => {
    return await ctx.runAction(api.requestMetadata.fromAction, {});
  },
});

// V8 action calling nested node action
export const fromV8ActionCallingNodeAction = action({
  args: {},
  handler: async (ctx): Promise<RequestMetadata> => {
    return await ctx.runAction(api.nodeActions.fromNodeAction, {});
  },
});

// Mutation calling nested mutation
export const fromMutationCallingMutation = mutation({
  args: {},
  handler: async (ctx): Promise<RequestMetadata> => {
    return await ctx.runMutation(api.requestMetadata.fromMutation, {});
  },
});
