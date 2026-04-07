"use node";

import { action } from "./_generated/server";
import { components } from "./_generated/api";
import { createFunctionHandle } from "convex/server";

export const nodeActionCallingComponentQuery = action({
  args: {},
  handler: async (ctx) => {
    return await ctx.runQuery(components.component.fileStorage.list);
  },
});

export const nodeActionCallingComponentMutation = action({
  args: {},
  handler: async (ctx) => {
    return await ctx.runMutation(components.component.scheduler.sendMessage, {
      message: "hello",
    });
  },
});

export const nodeActionCallingComponentAction = action({
  args: {},
  handler: async (ctx) => {
    return await ctx.runAction(components.searchComponent.foods.populate);
  },
});

export const nodeActionSchedulingInComponent = action({
  args: {},
  handler: async (ctx) => {
    await ctx.scheduler.runAfter(
      0,
      components.component.scheduler.sendMessage,
      {
        message: "hello",
      },
    );
  },
});

export const nodeActionCreateFunctionHandle = action({
  args: {},
  handler: async (ctx) => {
    const functionHandle = await createFunctionHandle(
      components.component.scheduler.sendMessage,
    );
    await ctx.scheduler.runAfter(0, functionHandle, { message: "hello" });
    return await ctx.runMutation(functionHandle, { message: "hello" });
  },
});
