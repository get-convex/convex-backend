import { v } from "convex/values";
import { asyncMap } from "modern-async";
import OpenAI from "openai";
import { internal } from "../_generated/api";
import { Id } from "../_generated/dataModel";
import {
  internalAction,
  internalMutation,
  internalQuery,
} from "../_generated/server";
import { paginate } from "../helpers";

export const embedAll = internalAction({
  args: {},
  handler: async (ctx) => {
    await paginate(ctx, "documents", 20, async (documents) => {
      await ctx.runAction(internal.ingest.embed.embedList, {
        documentIds: documents.map((doc) => doc._id),
      });
    });
  },
});

export const embedList = internalAction({
  args: {
    documentIds: v.array(v.id("documents")),
  },
  handler: async (ctx, { documentIds }) => {
    const chunks = (
      await asyncMap(documentIds, (documentId) =>
        ctx.runQuery(internal.ingest.embed.chunksNeedingEmbedding, {
          documentId,
        }),
      )
    ).flat();

    const embeddings = await embedTexts(chunks.map((chunk) => chunk.text));
    await asyncMap(embeddings, async (embedding, i) => {
      const { _id: chunkId } = chunks[i];
      await ctx.runMutation(internal.ingest.embed.addEmbedding, {
        chunkId,
        embedding,
      });
    });
  },
});

export const chunksNeedingEmbedding = internalQuery({
  handler: async (ctx, { documentId }: { documentId: Id<"documents"> }) => {
    const chunks = await ctx.db
      .query("chunks")
      .withIndex("byDocumentId", (q) => q.eq("documentId", documentId))
      .collect();
    return chunks.filter((chunk) => chunk.embeddingId === null);
  },
});

export const addEmbedding = internalMutation({
  handler: async (
    ctx,
    { chunkId, embedding }: { chunkId: Id<"chunks">; embedding: number[] },
  ) => {
    const embeddingId = await ctx.db.insert("embeddings", {
      embedding,
      chunkId,
    });
    await ctx.db.patch(chunkId, { embeddingId });
  },
});

export async function embedTexts(texts: string[]) {
  if (texts.length === 0) return [];
  const openai = new OpenAI();
  const { data } = await openai.embeddings.create({
    input: texts,
    model: "text-embedding-ada-002",
  });
  return data.map(({ embedding }) => embedding);
}
