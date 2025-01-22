import { action, mutation, query } from "../_generated/server";
import {
  customAction,
  customMutation,
  customQuery,
} from "convex-helpers/server/customFunctions";
import {
  SessionId,
  SessionIdArg,
  runSessionFunctions,
} from "convex-helpers/server/sessions";
import { QueryCtx } from "../_generated/server";

async function getUser(ctx: QueryCtx, sessionId: SessionId) {
  const user = await ctx.db
    .query("users")
    .withIndex("by_sessionId", (q) => q.eq("sessionId", sessionId))
    .unique();
  return user;
}

export const queryWithSession = customQuery(query, {
  args: SessionIdArg,
  input: async (ctx, { sessionId }) => {
    const user = await getUser(ctx, sessionId);
    return { ctx: { ...ctx, user, sessionId }, args: {} };
  },
});

export const mutationWithSession = customMutation(mutation, {
  args: SessionIdArg,
  input: async (ctx, { sessionId }) => {
    const user = await getUser(ctx, sessionId);
    return { ctx: { ...ctx, user, sessionId }, args: {} };
  },
});

export const actionWithSession = customAction(action, {
  args: SessionIdArg,
  input: async (ctx, { sessionId }) => {
    return {
      ctx: {
        ...ctx,
        ...runSessionFunctions(ctx, sessionId),
        sessionId,
      },
      args: {},
    };
  },
});
