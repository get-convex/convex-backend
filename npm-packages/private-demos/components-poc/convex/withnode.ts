"use node";

import { createFunctionHandle } from "convex/server";
import { action } from "./_generated/server";
import { api } from "./_generated/api";

export const run = action({
  args: {},
  handler: async (ctx) => {
    const funcHandle = await createFunctionHandle(api.messages.send);
    const message = { body: "hello", author: "me" };
    await ctx.scheduler.runAfter(0, funcHandle, message);
    await ctx.runMutation(funcHandle, message);
  },
});
