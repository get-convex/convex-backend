import { action, mutation } from "./_generated/server";
import { query } from "./_generated/server";
import { components } from "./_generated/api";
import { Doc } from "./_generated/dataModel";
import { v } from "convex/values";
import type { ComponentApi } from "../examples/waitlist@name-with-dashes/_generated/component.js";

const waitlist = components.waitlist satisfies ComponentApi;

export const list = query(async (ctx): Promise<Doc<"messages">[]> => {
  const result = await ctx.runQuery(waitlist.index.sayGoodbyeFromQuery, {});
  console.log(result);
  return await ctx.db.query("messages").collect();
});

export const send = mutation(
  async (ctx, { body, author }: { body: string; author: string }) => {
    const message = { body, author };
    await ctx.db.insert("messages", message);
  },
);

export const save = action({
  args: { message: v.string() },
  returns: v.string(),
  handler: async (ctx, { message }) => {
    return ctx.runAction(waitlist.index.storeInFile, { message });
  },
});

export const componentTest = action(async (ctx) => {
  console.log("calling into component...");
  const response = await ctx.runAction(waitlist.index.repeatMessage, {
    message: "hello",
    n: 3,
  });
  console.log("received response from component:", response);
  return response;
});

export const scheduleSendWaitlistMessage = mutation(async (ctx) => {
  console.log("scheduling message");
  await ctx.scheduler.runAfter(30 * 1000, waitlist.index.scheduleMessage, {});
  console.log(await ctx.db.system.query("_scheduled_functions").collect());
  return "scheduled";
});

export const testPartialRollback = mutation(async (ctx) => {
  const initialResult = await ctx.runQuery(waitlist.index.latestWrite, {});
  console.log(initialResult);
  await ctx.runMutation(waitlist.index.writeSuccessfully, {
    text: "hello",
  });
  try {
    await ctx.runMutation(waitlist.index.writeThenFail, {
      text: "world",
    });
  } catch (e) {
    console.log("caught error", e);
  }
  const result = await ctx.runQuery(waitlist.index.latestWrite, {});
  console.log(result);
});
