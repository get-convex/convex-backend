"use node";

import { api } from "../_generated/api";
import { Id } from "../_generated/dataModel";
import { action, internalAction } from "../_generated/server";

export const hello = action({
  handler: async (_, { somebody }: { somebody: string }) => {
    console.log(`Hello, ${somebody}!`);
    return `Aloha, ${somebody}!`;
  },
});

export const logPhilosophically = action({
  args: {},
  handler: async () => {
    console.log(
      "A person cannot walk in the same river twice, " +
        "for it is not the same river and they are not the same person.",
    );
    return "Success";
  },
});

export const returnInt64 = action({
  args: {},
  handler: async () => {
    return BigInt(1);
  },
});

export const returnSet = action({
  args: {},
  handler: async () => {
    return new Set(["hello", "world"]);
  },
});

export const returnMap = action({
  args: {},
  handler: async () => {
    return new Map([["key", "value"]]);
  },
});

export const userError = action({
  args: {},
  handler: async () => {
    throw new Error("I failed you!");
  },
});

export const scheduling = action({
  args: {},
  handler: async (ctx): Promise<Id<"_scheduled_functions">> => {
    const jobId = await ctx.scheduler.runAfter(0, api.actions.simple.hello, {
      somebody: "scheduler",
    });
    return jobId;
  },
});

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export const userTimeout = action({
  args: {},
  handler: async () => {
    await sleep(60 * 60 * 1000);
    return "Success";
  },
});

export const deadlock = action({
  args: {},
  handler: async () => {
    return new Promise(() =>
      setTimeout(() => {
        // intentionally empty.
      }, 500),
    );
  },
});

export const convexCloud = action({
  args: {},
  handler: async () => {
    return process.env.CONVEX_CLOUD_URL;
  },
});

export const convexSite = action({
  args: {},
  handler: async () => {
    return process.env.CONVEX_SITE_URL;
  },
});

export const internalUhOh = internalAction({
  args: {},
  handler: async () => {
    return "ruh roh";
  },
});

export const consoleTime = action({
  args: {},
  handler: async () => {
    console.time();
    await sleep(100);
    console.timeLog(); // default: Xms
    await sleep(100);
    console.timeEnd(); // default: Xms

    console.time("foo");
    await sleep(100);
    console.time("foo"); // Timer "foo" already exists
    console.timeLog("foo", "bar", "baz"); // foo: Xms bar baz
    await sleep(100);
    console.timeEnd("foo"); // foo: Xms
  },
});

export const actionCallsWithBigArgument = action({
  args: {},
  handler: async (ctx) => {
    const bigString = "a".repeat(6_050_000);
    await ctx.runQuery(api.basic.doNothing, { x: bigString } as any);
  },
});

export const nodeAction = action({
  args: {},
  handler: async () => {
    console.log("INNER");
  },
});

export const actionCallAction = action({
  args: {},
  handler: async (ctx) => {
    console.log("OUTER 1");
    await ctx.runAction(api.actions.simple.nodeAction);
    console.log("OUTER 2");
    await ctx.runAction(api.actions.simple.nodeAction);
    console.log("OUTER 3");
  },
});
