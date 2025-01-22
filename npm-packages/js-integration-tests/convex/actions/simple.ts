"use node";

import { api } from "../_generated/api";
import { Id } from "../_generated/dataModel";
import { action, internalAction } from "../_generated/server";
console.log("Log at import time");

export const hello = action(async (_, { somebody }: { somebody: string }) => {
  console.log(`Hello, ${somebody}!`);
  return `Aloha, ${somebody}!`;
});

export const logPhilosophically = action(async () => {
  console.log(
    "A person cannot walk in the same river twice, " +
      "for it is not the same river and they are not the same person.",
  );
  return "Success";
});

export const returnInt64 = action(async () => {
  return BigInt(1);
});

export const returnSet = action(async () => {
  return new Set(["hello", "world"]);
});

export const returnMap = action(async () => {
  return new Map([["key", "value"]]);
});

export const userError = action(async () => {
  throw new Error("I failed you!");
});

export const scheduling = action(
  async (ctx): Promise<Id<"_scheduled_functions">> => {
    const jobId = await ctx.scheduler.runAfter(0, api.actions.simple.hello, {
      somebody: "scheduler",
    });
    return jobId;
  },
);

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export const userTimeout = action(async () => {
  await sleep(60 * 60 * 1000);
  return "Success";
});

export const deadlock = action(async () => {
  return new Promise(() =>
    setTimeout(() => {
      // intentionally empty.
    }, 500),
  );
});

export const convexCloud = action(async () => {
  return process.env.CONVEX_CLOUD_URL;
});

export const convexSite = action(async () => {
  return process.env.CONVEX_SITE_URL;
});

export const internalUhOh = internalAction(async () => {
  return "ruh roh";
});

export const consoleTime = action(async () => {
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
});

export const actionCallsWithBigArgument = action(async (ctx) => {
  const bigString = "a".repeat(6_050_000);
  await ctx.runQuery(api.basic.doNothing, { x: bigString } as any);
});
