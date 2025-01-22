import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  messages: defineTable({
    isViewer: v.boolean(),
    sessionId: v.string(),
    text: v.string(),
  }).index("bySessionId", ["sessionId"]),
  documents: defineTable({
    url: v.string(),
    text: v.string(),
  }).index("byUrl", ["url"]),
  chunks: defineTable({
    documentId: v.id("documents"),
    text: v.string(),
    embeddingId: v.union(v.id("embeddings"), v.null()),
  })
    .index("byDocumentId", ["documentId"])
    .index("byEmbeddingId", ["embeddingId"]),
  embeddings: defineTable({
    embedding: v.array(v.number()),
    chunkId: v.id("chunks"),
  })
    .index("byChunkId", ["chunkId"])
    .vectorIndex("byEmbedding", {
      vectorField: "embedding",
      dimensions: 1536,
    }),
});
