import { v } from "convex/values";
import { map } from "modern-async";
import OpenAI from "openai";
import {
  internalAction,
  internalMutation,
  internalQuery,
} from "./_generated/server";
import { embedTexts } from "./ingest/embed";
import { internal } from "./_generated/api";
import { Id } from "./_generated/dataModel";

export const answer = internalAction({
  args: {
    sessionId: v.string(),
  },
  handler: async (ctx, { sessionId }) => {
    const messages = await ctx.runQuery(internal.serve.getMessages, {
      sessionId,
    });
    const lastUserMessage = messages.at(-1)!.text;

    const [embedding] = await embedTexts([lastUserMessage]);

    const searchResults = await ctx.vectorSearch("embeddings", "byEmbedding", {
      vector: embedding,
      limit: 8,
    });

    const relevantDocuments = await map(
      searchResults,
      async ({ _id: embeddingId }) =>
        await ctx.runQuery(internal.serve.getChunk, { embeddingId }),
    );

    const messageId = await ctx.runMutation(internal.serve.addBotMessage, {
      sessionId,
    });

    try {
      const openai = new OpenAI();
      const stream = await openai.chat.completions.create({
        model: "gpt-4-32k",
        stream: true,
        messages: [
          {
            role: "system",
            content:
              "Answer the user question based on the provided documents " +
              "or report that the question cannot be answered based on " +
              "these documents. Keep the answer informative but brief, " +
              "do not enumerate all possibilities.",
          },
          ...(relevantDocuments.map(({ text }) => ({
            role: "system",
            content: "Relevant document:\n\n" + text,
          })) as OpenAI.ChatCompletionMessageParam[]),
          ...(messages.map(({ isViewer, text }) => ({
            role: isViewer ? "user" : "assistant",
            content: text,
          })) as OpenAI.ChatCompletionMessageParam[]),
        ],
      });
      let text = "";
      for await (const { choices } of stream) {
        const chunk = choices[0].delta.content;
        if (typeof chunk === "string" && chunk.length > 0) {
          text += choices[0].delta.content;
          await ctx.runMutation(internal.serve.updateBotMessage, {
            messageId,
            text,
          });
        }
      }
    } catch (error: any) {
      await ctx.runMutation(internal.serve.updateBotMessage, {
        messageId,
        text: "I cannot reply at this time. Reach out to the team on Discord",
      });
      throw error;
    }
  },
});

export const getMessages = internalQuery(
  async (ctx, { sessionId }: { sessionId: string }) => {
    return await ctx.db
      .query("messages")
      .withIndex("bySessionId", (q) => q.eq("sessionId", sessionId))
      .collect();
  },
);

export const getChunk = internalQuery(
  async (ctx, { embeddingId }: { embeddingId: Id<"embeddings"> }) => {
    return (await ctx.db
      .query("chunks")
      .withIndex("byEmbeddingId", (q) => q.eq("embeddingId", embeddingId))
      .unique())!;
  },
);

export const addBotMessage = internalMutation(
  async (ctx, { sessionId }: { sessionId: string }) => {
    return await ctx.db.insert("messages", {
      isViewer: false,
      text: "",
      sessionId,
    });
  },
);

export const updateBotMessage = internalMutation(
  async (
    ctx,
    { messageId, text }: { messageId: Id<"messages">; text: string },
  ) => {
    await ctx.db.patch(messageId, { text });
  },
);
