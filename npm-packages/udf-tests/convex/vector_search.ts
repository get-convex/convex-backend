import { v } from "convex/values";
import { api } from "./_generated/api";
import { action, mutation, query } from "./_generated/server";
import { assert } from "chai";

export const populate = mutation({
  args: {},
  handler: async ({ db }) => {
    const vectorDocs = [
      {
        vector: [1, 2, 3, 4],
        filterA: "A",
        filterB: true,
        id: "doc1",
      },
      {
        vector: [1, 2, 3, 4],
        filterA: "B",
        filterB: true,
        id: "doc2",
      },
      {
        vector: [1, 2, 3, 4],
        filterA: "C",
        filterB: false,
        id: "doc3",
      },
      {
        vector: [1, 2, 3, 4],
        filterA: "Z",
        filterB: true,
        id: "doc4",
      },
    ];
    for (const vectorDoc of vectorDocs) {
      await db.insert("vectorTable", vectorDoc);
    }
  },
});

export const getDocuments = query({
  args: {
    ids: v.array(v.id("vectorTable")),
  },
  handler: async (ctx, args) => {
    const result = [];
    for (const id of args.ids) {
      const doc = await ctx.db.get(id);
      if (doc !== null) {
        result.push(doc);
      }
    }
    return result;
  },
});

export const multiFieldFilter = action({
  args: {},
  handler: async (ctx) => {
    // should return the first 3 docs
    const result = await ctx.vectorSearch("vectorTable", "vector", {
      vector: [1, 2, 3, 4],
      filter: (q) =>
        q.or(
          q.or(q.eq("filterA", "A"), q.eq("filterA", "B")),
          q.eq("filterB", false),
        ),
    });
    const docs = await ctx.runQuery(api.vector_search.getDocuments, {
      ids: result.map((r) => r._id),
    });
    assert.deepEqual(["doc1", "doc2", "doc3"], docs.map((d) => d.id).sort());
    return "success";
  },
});

export const multiValueFilter = action({
  args: {},
  handler: async (ctx) => {
    const result = await ctx.vectorSearch("vectorTable", "vector", {
      vector: [1, 2, 3, 4],
      filter: (q) => q.or(q.eq("filterA", "A"), q.eq("filterA", "B")),
    });
    const docs = await ctx.runQuery(api.vector_search.getDocuments, {
      ids: result.map((r) => r._id),
    });
    assert.deepEqual(["doc1", "doc2"], docs.map((d) => d.id).sort());
    return "success";
  },
});

export const singleValueFilter = action({
  args: {},
  handler: async (ctx) => {
    const result = await ctx.vectorSearch("vectorTable", "vector", {
      vector: [1, 2, 3, 4],
      filter: (q) => q.eq("filterA", "A"),
    });
    const docs = await ctx.runQuery(api.vector_search.getDocuments, {
      ids: result.map((r) => r._id),
    });
    assert.deepEqual(["doc1"], docs.map((d) => d.id).sort());
    return "success";
  },
});

export const invalidFilter = action({
  args: {},
  handler: async (ctx) => {
    await ctx.vectorSearch("vectorTable", "vector", {
      vector: [1, 2, 3, 4],
      // @ts-expect-error -- this is invalid and shouldn't compile and also
      // should error
      filter: (q) => q.eq("filterB", q.eq("filterA", "A")),
    });
    return "failure";
  },
});

export const noFilter = action({
  args: {},
  handler: async (ctx) => {
    const result = await ctx.vectorSearch("vectorTable", "vector", {
      vector: [1, 2, 3, 4],
    });
    const docs = await ctx.runQuery(api.vector_search.getDocuments, {
      ids: result.map((r) => r._id),
    });
    assert.deepEqual(
      ["doc1", "doc2", "doc3", "doc4"],
      docs.map((d) => d.id).sort(),
    );
    return "success";
  },
});
