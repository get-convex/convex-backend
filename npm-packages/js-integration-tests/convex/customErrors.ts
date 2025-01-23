import { ConvexError } from "convex/values";
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import { action, mutation, query } from "./_generated/server";
import { api, components } from "./_generated/api";

export const queryThrowingConvexError = query(async () => {
  throw new ConvexError("Boom boom bop");
});

export const mutationThrowingConvexError = mutation(async () => {
  throw new ConvexError({ message: "Boom boom bop", code: 123n });
});

export const actionThrowingConvexError = action(async () => {
  throw new ConvexError("Boom boom bop");
});

export const queryThrowingNormalError = query(async () => {
  throw new Error("Normal error");
});

class FooError extends ConvexError<{ message: string; code: bigint }> {
  name = "FooError";
  constructor(message: string) {
    super({ message, code: BigInt(123) });
  }
}

export const queryThrowingConvexErrorSubclass = query(async () => {
  throw new FooError("Boom boom bop");
});

export const actionCallingQueryThrowingConvexError = action(async (ctx) => {
  await ctx.runQuery(api.customErrors.queryThrowingConvexError);
});

export const actionCallingQueryThrowingConvexErrorSubclass = action(
  async (ctx) => {
    await ctx.runQuery(api.customErrors.queryThrowingConvexErrorSubclass);
  },
);

export const actionCallingMutationThrowingConvexError = action(async (ctx) => {
  await ctx.runMutation(api.customErrors.mutationThrowingConvexError);
});

export const actionCallingActionThrowingConvexError = action(async (ctx) => {
  await ctx.runAction(api.customErrors.actionThrowingConvexError);
});

export const actionCallingActionCallingMutationThrowingConvexError = action(
  async (ctx) => {
    await ctx.runAction(
      api.customErrors.actionCallingMutationThrowingConvexError,
    );
  },
);

export const actionCallingNodeActionThrowingConvexError = action(
  async (ctx) => {
    await ctx.runAction(
      api.customErrorsNodeActions.nodeActionThrowingConvexError,
    );
  },
);

export const mutationSchedulingActionCallingMutation = mutation(async (ctx) => {
  await ctx.scheduler.runAfter(
    0,
    api.customErrors.actionCallingMutationThrowingConvexErrorAndSavingResult,
  );
});

export const actionCallingMutationThrowingConvexErrorAndSavingResult = action(
  async (ctx) => {
    try {
      await ctx.runMutation(api.customErrors.mutationThrowingConvexError);
    } catch (error) {
      await ctx.runMutation(api.customErrors.helperMutation, {
        name: error instanceof ConvexError ? "ConvexError" : "Error",
      });
    }
  },
);

export const helperMutation = mutation(
  async (ctx, { name }: { name: string }) => {
    await ctx.db.insert("users", { name });
  },
);

export const componentQueryThrowingConvexError = query(async (ctx) => {
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  await ctx.runQuery(components.component.errors.throwConvexError, {});
});

export const componentQueryThrowingError = query(async (ctx) => {
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  await ctx.runQuery(components.component.errors.throwError, {});
});

export const actionCallingComponentQueryThrowingConvexError = action(
  async (ctx) => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    await ctx.runQuery(components.component.errors.throwConvexError, {});
  },
);
