// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import { createFunctionHandle } from "convex/server";
import { api } from "./_generated/api";

// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import { action, mutation, query } from "./_generated/server";
import { components } from "./_generated/api";
import { v } from "convex/values";

export const hello = query(async (_ctx) => {
  return "hello";
});

export const functionHandleQuery = query(async (ctx) => {
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  const componentHandle = await ctx.runQuery(
    components.component.functionHandles.fromQuery,
    {},
  );
  const appHandle: string = await createFunctionHandle(api.component.hello);
  return {
    appHandle,
    componentHandle,
  };
});

export const functionHandleAction = action(async (ctx) => {
  const componentHandle = await ctx.runAction(
    components.component.functionHandles.fromAction,
    {},
  );
  const appHandle: string = await createFunctionHandle(api.component.hello);
  return {
    appHandle,
    componentHandle,
  };
});

export const queryCallsHandles = query(async (ctx) => {
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  const queryHandle = await ctx.runQuery(
    components.component.functionHandles.getInternalHandle,
    { functionType: "query" },
  );
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  const result = await ctx.runQuery(queryHandle, { a: 1, b: 2 });
  if (result !== 3) {
    throw new Error("Query handle did not return the correct value");
  }
});

export const mutationCallsHandles = mutation(async (ctx) => {
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  const queryHandle = await ctx.runQuery(
    components.component.functionHandles.getInternalHandle,
    { functionType: "query" },
  );
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  const queryResult = await ctx.runQuery(queryHandle, { a: 1, b: 2 });
  if (queryResult !== 3) {
    throw new Error("Query handle did not return the correct value");
  }

  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  const mutationHandle = await ctx.runQuery(
    components.component.functionHandles.getInternalHandle,
    { functionType: "mutation" },
  );
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  const mutationResult = await ctx.runMutation(mutationHandle, { a: 1, b: 2 });
  if (mutationResult !== 2) {
    throw new Error("Mutation handle did not return the correct value");
  }
});

export const actionCallsHandles = action(async (ctx) => {
  const queryHandle: any = await ctx.runQuery(
    components.component.functionHandles.getInternalHandle,
    { functionType: "query" },
  );
  const queryResult = await ctx.runQuery(queryHandle, { a: 1, b: 2 });
  if (queryResult !== 3) {
    throw new Error("Query handle did not return the correct value");
  }

  const mutationHandle: any = await ctx.runQuery(
    components.component.functionHandles.getInternalHandle,
    { functionType: "mutation" },
  );
  const mutationResult = await ctx.runMutation(mutationHandle, { a: 1, b: 2 });
  if (mutationResult !== 2) {
    throw new Error("Mutation handle did not return the correct value");
  }

  const actionHandle: any = await ctx.runQuery(
    components.component.functionHandles.getInternalHandle,
    { functionType: "action" },
  );
  const actionResult = await ctx.runAction(actionHandle, { a: 1, b: 2 });
  if (actionResult !== 0.5) {
    throw new Error("Action handle did not return the correct value");
  }
});

export const passHandleToScheduler = mutation(async (ctx) => {
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  const queryHandle = await ctx.runQuery(
    components.component.functionHandles.getInternalHandle,
    { functionType: "query" },
  );
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  await ctx.scheduler.runAfter(0, queryHandle, { a: 1, b: 2 });

  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  const mutationHandle = await ctx.runQuery(
    components.component.functionHandles.getInternalHandle,
    { functionType: "mutation" },
  );
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  await ctx.scheduler.runAfter(0, mutationHandle, { a: 1, b: 2 });

  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  const actionHandle = await ctx.runQuery(
    components.component.functionHandles.getInternalHandle,
    { functionType: "action" },
  );
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  await ctx.scheduler.runAfter(0, actionHandle, { a: 1, b: 2 });
});

export const populateFoods = action({
  args: {},
  handler: async (ctx) => {
    return await ctx.runAction(components.searchComponent.foods.populate, {});
  },
});

export const vectorSearchInComponent = action({
  args: { embedding: v.array(v.float64()), cuisine: v.string() },
  handler: async (ctx, args) => {
    return await ctx.runAction(
      components.searchComponent.vectorActionV8.vectorSearch,
      args,
    );
  },
});
