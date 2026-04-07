import { v } from "convex/values";
import { api } from "./_generated/api";
import { action, mutation } from "./_generated/server";

declare const Convex: {
  syscall: (op: string, jsonArgs: string) => string;
  asyncSyscall: (op: string, jsonArgs: string) => Promise<string>;
  jsSyscall: (op: string, args: Record<string, any>) => any;
};

export const getCloudUrl = action({
  args: {},
  handler: async () => {
    return process.env.CONVEX_CLOUD_URL;
  },
});

export const getSiteUrl = action({
  args: {},
  handler: async () => {
    return process.env.CONVEX_SITE_URL;
  },
});

export const insertObject = action({
  handler: async ({ runMutation, runQuery }, args) => {
    await runMutation(api.basic.insertObject, args);
    const count: number = await runQuery(api.basic.count, {});
    return count;
  },
});

export const schedule = action({
  handler: async ({ scheduler }, args) => {
    await scheduler.runAfter(0, api.basic.insertObject, args);
  },
});

export const sleep = action({
  args: { ms: v.number() },
  handler: async (_ctx, { ms }) => {
    await new Promise((resolve) => setTimeout(resolve, ms));
  },
});

export const inc = mutation({
  args: {},
  handler: async (ctx) => {
    const object = await ctx.db.query("objects").first();
    await ctx.db.patch(object!._id, { x: object!.x + 1 });
  },
});

export const occAction = action({
  args: {},
  handler: async (ctx) => {
    await ctx.runMutation(api.basic.insertObject, { x: 1 });
    await Promise.all(
      new Array(16).fill(0).map(async () => {
        await ctx.runMutation(api.action.inc, {});
      }),
    );
  },
});

export const innerSystemErrorAction = action({
  args: {},
  handler: async (ctx) => {
    await ctx.runQuery(api.adversarial.throwSystemError);
  },
});

export const systemErrorAction = action({
  args: {},
  handler: async () => {
    try {
      await (async () => {
        await new Promise((resolve) => setTimeout(resolve, 20));
        Convex.syscall("throwSystemError", "{}");
      })();
    } catch {
      // This should not swallow the system error
    }
  },
});

export const innerUncatchableDeveloperErrorAction = action({
  args: {},
  handler: async (ctx) => {
    await ctx.runQuery(api.adversarial.throwUncatchableDeveloperError);
  },
});

export const uncatchableDeveloperErrorAction = action({
  args: {},
  handler: async () => {
    try {
      await (async () => {
        await new Promise((resolve) => setTimeout(resolve, 20));
        Convex.jsSyscall("idonotexistandicannotlie", {});
      })();
    } catch {
      // This should not swallow the uncatchable developer error
    }
  },
});
