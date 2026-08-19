// Ordinary Convex functions: nothing here knows it is running inside Felix.
// The `ctx` these handlers receive comes from `convex-test`, which implements
// the real database, index, scheduler, and function-calling semantics in JS.
import { v } from "convex/values";

import { api, internal } from "./_generated/api";
import {
  action,
  internalMutation,
  internalQuery,
  mutation,
  query,
} from "./_generated/server";

export const list = query({
  args: {
    channel: v.string(),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, { channel, limit }) => {
    const messages = await ctx.db
      .query("messages")
      .withIndex("by_channel", (q) => q.eq("channel", channel))
      .order("desc")
      .take(limit ?? 50);

    return messages.map((message) => ({
      id: message._id,
      author: message.author,
      body: message.body,
    }));
  },
});

export const get = query({
  args: { id: v.id("messages") },
  handler: async (ctx, { id }) => ctx.db.get("messages", id),
});

export const stats = query({
  args: { channel: v.string() },
  handler: async (ctx, { channel }) => {
    const messages = await ctx.db
      .query("messages")
      .withIndex("by_channel", (q) => q.eq("channel", channel))
      .collect();

    return {
      channel,
      messageCount: messages.length,
      authorCount: new Set(messages.map((message) => message.author)).size,
      characters: messages.reduce(
        (total, message) => total + message.body.length,
        0,
      ),
    };
  },
});

export const search = query({
  args: { channel: v.string(), needle: v.string() },
  handler: async (ctx, { channel, needle }) => {
    const messages = await ctx.db
      .query("messages")
      .withIndex("by_channel", (q) => q.eq("channel", channel))
      // The guest has to support server-side filters, which is what this
      // fixture is here to exercise.
      // eslint-disable-next-line @convex-dev/no-filter-in-query
      .filter((q) => q.neq(q.field("body"), ""))
      .collect();

    const lowered = needle.toLowerCase();
    return messages
      .filter((message) => message.body.toLowerCase().includes(lowered))
      .map((message) => message._id);
  },
});

export const send = mutation({
  args: {
    channel: v.string(),
    author: v.string(),
    body: v.string(),
    tags: v.optional(v.array(v.string())),
  },
  handler: async (ctx, { channel, author, body, tags }) => {
    if (body.length === 0) {
      throw new Error("message body must not be empty");
    }

    return ctx.db.insert("messages", {
      channel,
      author,
      body,
      tags: tags ?? [],
      edited: false,
    });
  },
});

export const edit = mutation({
  args: { id: v.id("messages"), body: v.string() },
  handler: async (ctx, { id, body }) => {
    const existing = await ctx.db.get("messages", id);
    if (existing === null) {
      throw new Error(`message ${id} does not exist`);
    }

    await ctx.db.patch("messages", id, { body, edited: true });
    return { id, previousBody: existing.body };
  },
});

export const remove = mutation({
  args: { id: v.id("messages") },
  handler: async (ctx, { id }) => {
    const existing = await ctx.db.get("messages", id);
    if (existing === null) {
      return false;
    }

    await ctx.db.delete("messages", id);
    return true;
  },
});

export const purgeChannel = mutation({
  args: { channel: v.string() },
  handler: async (ctx, { channel }) => {
    const messages = await ctx.db
      .query("messages")
      .withIndex("by_channel", (q) => q.eq("channel", channel))
      .collect();

    for (const message of messages) {
      await ctx.db.delete("messages", message._id);
    }

    return messages.length;
  },
});

export const linesForSummary = internalQuery({
  args: { channel: v.string() },
  handler: async (ctx, { channel }) => {
    const messages = await ctx.db
      .query("messages")
      .withIndex("by_channel", (q) => q.eq("channel", channel))
      .collect();

    return messages.map((message) => `${message.author}: ${message.body}`);
  },
});

export const recordSummary = internalMutation({
  args: { channel: v.string(), summary: v.string() },
  handler: async (ctx, { channel, summary }) => {
    const existing = await ctx.db
      .query("summaries")
      .withIndex("by_channel", (q) => q.eq("channel", channel))
      .first();

    if (existing !== null) {
      await ctx.db.patch("summaries", existing._id, { summary });
      return existing._id;
    }

    return ctx.db.insert("summaries", { channel, summary });
  },
});

export const summarize = action({
  args: { channel: v.string(), summarizerUrl: v.string() },
  handler: async (ctx, { channel, summarizerUrl }) => {
    const lines: string[] = await ctx.runQuery(
      internal.messages.linesForSummary,
      { channel },
    );
    if (lines.length === 0) {
      return { channel, summary: null, summaryId: null };
    }

    const response = await fetch(summarizerUrl, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ channel, lines }),
    });

    if (!response.ok) {
      throw new Error(`summarizer failed with status ${response.status}`);
    }

    const summary = (await response.text()).slice(0, 280);
    const summaryId = await ctx.runMutation(internal.messages.recordSummary, {
      channel,
      summary,
    });
    return { channel, summary, summaryId };
  },
});

export const importFromUrl = action({
  args: { channel: v.string(), sourceUrl: v.string() },
  handler: async (ctx, { channel, sourceUrl }) => {
    const response = await fetch(sourceUrl);
    const payload = (await response.json()) as {
      author: string;
      body: string;
    }[];

    const ids = [];
    for (const entry of payload) {
      ids.push(
        await ctx.runMutation(api.messages.send, {
          channel,
          author: entry.author,
          body: entry.body,
        }),
      );
    }

    return { imported: ids.length, ids };
  },
});

export const fanOutFetches = action({
  args: { urls: v.array(v.string()) },
  handler: async (_ctx, { urls }) => {
    const responses = await Promise.all(urls.map((url) => fetch(url)));
    return Promise.all(
      responses.map(async (response) => ({
        status: response.status,
        length: (await response.text()).length,
      })),
    );
  },
});
