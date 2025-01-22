"use node";

import { ConvexError } from "convex/values";
import { action } from "./_generated/server";
import { api } from "./_generated/api";

export const nodeActionThrowingConvexError = action(async () => {
  throw new ConvexError("Boom boom bop");
});

export const nodeActionCallingMutationThrowingConvexError = action(
  async (ctx) => {
    await ctx.runMutation(api.customErrors.mutationThrowingConvexError);
  },
);

export const nodeActionCallingQueryThrowingConvexError = action(async (ctx) => {
  await ctx.runQuery(api.customErrors.queryThrowingConvexError);
});

export const nodeActionCallingQueryThrowingConvexErrorSubclass = action(
  async (ctx) => {
    await ctx.runQuery(api.customErrors.queryThrowingConvexErrorSubclass);
  },
);

export const nodeActionCallingActionThrowingConvexError = action(
  async (ctx) => {
    await ctx.runAction(api.customErrors.actionThrowingConvexError);
  },
);

export const nodeActionCallingNodeActionThrowingConvexError = action(
  async (ctx) => {
    await ctx.runAction(
      api.customErrorsNodeActions.nodeActionThrowingConvexError,
    );
  },
);
