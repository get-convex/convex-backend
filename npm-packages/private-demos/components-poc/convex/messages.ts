import { action, internalQuery, mutation } from "./_generated/server";
import { query } from "./_generated/server";
import { api, internal, components } from "./_generated/api";
import { Doc } from "./_generated/dataModel";
import { v } from "convex/values";
import { createFunctionHandle, FunctionHandle } from "convex/server";
import { functionValidator } from "./types";
import { add } from "@convex-dev/ratelimiter";

export const list = query({
  args: {},
  handler: async (ctx): Promise<Doc<"messages">[]> => {
    const result = await ctx.runQuery(
      components.waitlist.index.sayGoodbyeFromQuery,
      {},
    );
    console.log(result);
    return await ctx.db.query("messages").collect();
  },
});

export const getFunctionHandle = query({
  args: {},
  handler: async () => {
    const handle: string = await createFunctionHandle(api.messages.list);
    return handle;
  },
});

export const getChildFunctionHandle = query({
  args: {},
  handler: async () => {
    const handle: string = await createFunctionHandle(
      components.waitlist.index.listFiles,
    );
    return handle;
  },
});

export const getFunctionHandleAction = action({
  args: {},
  handler: async () => {
    const handle: string = await createFunctionHandle(api.messages.list);
    return handle as string;
  },
});

export const sumNumbers = internalQuery({
  args: {
    a: v.number(),
    b: v.number(),
  },
  handler: async (_, args) => {
    return args.a + args.b;
  },
});

export const getSumNumbers = query({
  args: {},
  handler: async () => {
    const handle: string = await createFunctionHandle(
      internal.messages.sumNumbers,
    );
    return handle;
  },
});

export const takeInHandle = query({
  args: {
    a: v.number(),
    b: v.number(),
    handle:
      functionValidator<
        FunctionHandle<"query", { a: number; b: number }, number>
      >(),
  },
  handler: async (ctx, args) => {
    const result = await ctx.runQuery(args.handle, { a: args.a, b: args.b });
    return result;
  },
});

export const storeHandle = mutation({
  args: {},
  handler: async (ctx) => {
    const handle = await createFunctionHandle(internal.messages.sumNumbers);
    await ctx.db.insert("functionHandles", {
      untyped: handle,
      typed: handle,
    });

    for await (const document of ctx.db.query("functionHandles")) {
      const typedResult = await ctx.runQuery(document.typed, {
        a: 2,
        b: 3,
      });
      const untypedResult = await ctx.runQuery(document.untyped, {
        a: 4,
        b: 5,
      });
      console.log({ typedResult, untypedResult });
    }
  },
});

export const send = mutation({
  handler: async (ctx, { body, author }: { body: string; author: string }) => {
    const result = await ctx.runMutation(
      components.ratelimiter.index.rateLimit,
      {
        name: "send",
        key: author,
      },
    );
    // TODO: Output validators need to support non-object types to have a
    // more precise union type here.
    if (!result.ok) {
      const waitTime = result.retryAt! - Date.now();
      const asSec = Math.round(waitTime / 1000).toFixed(2);
      throw new Error(`Rate limit exceeded, please try again in ${asSec}s.`);
    }
    console.log(result);
    const message = { body, author };
    await ctx.db.insert("messages", message);
  },
});

export const save = action({
  args: { message: v.string() },
  returns: v.string(),
  handler: async (ctx, { message }) => {
    return ctx.runAction(components.waitlist.index.storeInFile, { message });
  },
});

export const readSaved = action({
  args: { id: v.string() },
  returns: v.string(),
  handler: async (ctx, args) => {
    const files = await ctx.runQuery(api.messages.listFiles);
    console.log("files", files);
    const innerFiles = await ctx.runQuery(components.waitlist.index.listFiles);
    console.log("inner files", innerFiles);
    return ctx.runAction(components.waitlist.index.readFromFile, {
      id: args.id,
    });
  },
});

export const listFiles = query({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.system.query("_storage").collect();
  },
});

export const fileUploadUrl = mutation({
  args: {},
  returns: v.string(),
  handler: async (ctx) => {
    return ctx.runMutation(components.waitlist.index.fileUploadUrl, {});
  },
});

export const fileDownloadUrl = query({
  args: { id: v.string() },
  returns: v.string(),
  handler: async (ctx, { id }) => {
    return ctx.runQuery(components.waitlist.index.fileDownloadUrl, { id });
  },
});

export const componentTest = action({
  args: {},
  handler: async (ctx) => {
    console.log("calling into component...");
    const response = await ctx.runAction(
      components.waitlist.index.repeatMessage,
      {
        message: "hello",
        n: 3,
      },
    );
    console.log("received response from component:", response);
    return response;
  },
});

export const componentTest2 = action({
  args: {},
  handler: async (ctx) => {
    console.log("calling into component...");
    const response = await ctx.runAction(
      components.waitlist.actionDemo.demo,
      {},
    );
    console.log("received response from component:", response);
    return response;
  },
});

export const startCron = mutation({
  args: {},
  handler: async (ctx) => {
    console.log("starting cron job...");
    await ctx.runMutation(components.waitlist.index.scheduleMessage, {});
    console.log("cron job started");
  },
});

export const getMessageCount = query({
  args: {},
  handler: async (ctx) => {
    return ctx.runQuery(components.waitlist.index.getMessageCount, {});
  },
});

export const scheduleSendWaitlistMessage = mutation({
  args: {},
  handler: async (ctx) => {
    console.log("scheduling message");
    await ctx.scheduler.runAfter(
      30 * 1000,
      components.waitlist.index.scheduleMessage,
      {},
    );
    console.log(await ctx.db.system.query("_scheduled_functions").collect());
    return "scheduled";
  },
});

export const testPartialRollback = mutation({
  args: {},
  handler: async (ctx) => {
    const initialResult = await ctx.runQuery(
      components.waitlist.index.latestWrite,
      {},
    );
    console.log(initialResult);
    await ctx.runMutation(components.waitlist.index.writeSuccessfully, {
      text: "hello",
    });
    try {
      await ctx.runMutation(components.waitlist.index.writeThenFail, {
        text: "world",
      });
    } catch (e) {
      console.log("caught error", e);
    }
    const result = await ctx.runQuery(
      components.waitlist.index.latestWrite,
      {},
    );
    console.log(result);
  },
});

const rlClient = add(1, 2);
console.log(rlClient);
