// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import { mutation, query } from "./_generated/server";
import { components } from "./_generated/api";
import { PaginationOptions } from "convex/server";

export const sendMessage = mutation({
  handler: async (
    { db },
    { channel, text }: { channel: string; text: string },
  ) => {
    const message = { channel, text };
    const id = await db.insert("messages", message);
    return await db.get(id);
  },
});

export const listMessages = query({
  handler: async ({ db }, { channel }: { channel: string }) => {
    return await db
      .query("messages")
      .filter((q) => q.eq(q.field("channel"), channel))
      .collect();
  },
});

export const paginatedListMessagesByChannel = query({
  handler: async (
    { db },
    {
      paginationOpts,
      channel,
    }: { paginationOpts: PaginationOptions; channel: string },
  ) => {
    return await db
      .query("messages")
      .withIndex("by_creation_time")
      .filter((q) => q.eq(q.field("channel"), channel))
      .paginate(paginationOpts);
  },
});

export const paginatedListMessagesByCreationTime = query({
  handler: async (
    { db },
    { paginationOpts }: { paginationOpts: PaginationOptions },
  ) => {
    return await db
      .query("messages")
      .withIndex("by_channel")
      .paginate(paginationOpts);
  },
});

export const paginatedListMessagesWithExplicitPages = query({
  handler: async (
    { db },
    { paginationOpts }: { paginationOpts: PaginationOptions },
  ) => {
    const results = await db
      .query("messages")
      .withIndex("by_channel")
      .paginate(paginationOpts);
    return {
      ...results,
      page: results.page.map((doc, i) => ({
        ...doc,
        i,
      })),
    };
  },
});

export const paginatedListMessagesMaxRows = query({
  handler: async (
    { db },
    { paginationOpts }: { paginationOpts: PaginationOptions },
  ) => {
    const results = await db
      .query("messages")
      .withIndex("by_channel")
      .paginate({ ...paginationOpts, maximumRowsRead: 3 } as PaginationOptions);
    return {
      ...results,
      page: results.page.map((doc, i) => ({
        ...doc,
        i,
      })),
    };
  },
});

export const listMessagesInRange = query({
  handler: async (
    { db },
    {
      lower,
      lowerEqual,
      upper,
      upperEqual,
    }: {
      lower: string;
      lowerEqual: boolean;
      upper: string;
      upperEqual: boolean;
    },
  ) => {
    return await db
      .query("messages")
      .filter((q) =>
        q.and(
          (lowerEqual ? q.gte : q.gt)(q.field("channel"), lower),
          (upperEqual ? q.lte : q.lt)(q.field("channel"), upper),
        ),
      )
      .collect();
  },
});

export const partialRollback = mutation({
  args: {},
  handler: async (ctx) => {
    async function sendButFail(message: string) {
      try {
        // eslint-disable-next-line @typescript-eslint/ban-ts-comment
        // @ts-ignore
        await ctx.runMutation(components.component.transact.sendButFail, {
          message,
        });
      } catch {
        return;
      }
      throw new Error("expected error");
    }

    await sendButFail("hello fren");
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    const latest0 = await ctx.runQuery(
      components.component.transact.allMessages,
      {},
    );
    if (latest0.length !== 0) {
      throw new Error("expected empty");
    }

    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    await ctx.runMutation(components.component.transact.sendMessage, {
      message: "hello buddy",
    });

    await sendButFail("hello guy");

    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    const latest1 = await ctx.runQuery(
      components.component.transact.allMessages,
      {},
    );
    if (latest1.length !== 1 || latest1[0] !== "hello buddy") {
      throw new Error("expected 'hello buddy'");
    }
  },
});

export const messagesInComponent = query({
  args: {},
  handler: async (ctx): Promise<string> => {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    return await ctx.runQuery(components.component.transact.allMessages, {});
  },
});
