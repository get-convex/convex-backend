import { action, query } from "./_generated/server";

export const badDbIndex = query(async ({ db }) => {
  return await db
    .query("stagedIndexes")
    .withIndex("by_name" as any)
    .collect();
});

export const badSearchIndex = query(async ({ db }) => {
  return await db
    .query("stagedIndexes")
    .withSearchIndex("search_by_name" as never, (q) =>
      q.search("name" as never, "nipunn"),
    )
    .collect();
});

export const badVectorSearch = action(async (ctx) => {
  return await ctx.vectorSearch("stagedIndexes", "by_embedding" as never, {
    vector: [1, 2, 3],
  });
});
