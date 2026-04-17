import { v } from "convex/values";
import { action, internalQuery } from "./_generated/server";
import { internal } from "./_generated/api";

export const readInvocationMetadata = internalQuery({
  args: {},
  handler: async (ctx) => {
    const invocation = await ctx.meta.getInvocationContext();
    const metadata = invocation.metadata;
    return {
      requestId: invocation.requestId,
      correlationId:
        typeof metadata?.correlationId === "string"
          ? metadata.correlationId
          : null,
      phase: typeof metadata?.phase === "string" ? metadata.phase : "initial",
    };
  },
});

export const processOrder = action({
  args: { orderId: v.string() },
  handler: async (ctx, args) => {
    const invocation = await ctx.meta.getInvocationContext();
    const metadata = invocation.metadata;
    const correlationId =
      typeof metadata?.correlationId === "string"
        ? metadata.correlationId
        : `order:${args.orderId}`;

    return await ctx.runQuery(
      internal.invocationMetadata.readInvocationMetadata,
      {},
      {
        metadata: {
          correlationId,
          phase: "authorize",
        },
      },
    );
  },
});
