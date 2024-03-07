import { query, mutation } from "./_generated/server";
import { Cursor } from "convex/server";

export const populateSearch = mutation(async ({ db }) => {
  // BM25 ranks on number of matches terms, so create a range of them.
  // "a b" and "a c" should tie for last. Then we sort on creation time.
  const messages = ["a", "a a", "a a a", "a a a a", "a b", "a c"];
  for (const message of messages) {
    await db.insert("messages", { body: message });
  }
});

export const querySearch = query(
  async ({ db }, { query }: { query: string }) => {
    return db
      .query("messages")
      .withSearchIndex("by_body", (q) => q.search("body", query))
      .collect();
  },
);

export const paginatedSearch = query(
  async ({ db }, { cursor, query }: { cursor: Cursor; query: string }) => {
    return db
      .query("messages")
      .withSearchIndex("by_body", (q) => q.search("body", query))
      .paginate({ cursor, numItems: 1 });
  },
);

export const createDocumentAndSearchForIt = mutation(async ({ db }) => {
  await db.insert("messages", {
    body: "a",
  });
  return db
    .query("messages")
    .withSearchIndex("by_body", (q) => q.search("body", "a"))
    .collect();
});

/**
 * UDFs for error cases
 */

export const incorrectSearchField = query(async ({ db }) => {
  return db
    .query("messages")
    .withSearchIndex("by_body", (q: any) =>
      q.search("nonexistentField", "search query"),
    )
    .collect();
});

export const duplicateSearchFilters = query(async ({ db }) => {
  return db
    .query("messages")
    .withSearchIndex("by_body", (q: any) =>
      q.search("body", "search query1").search("body", "search query1"),
    )
    .collect();
});

export const incorrectFilterField = query(async ({ db }) => {
  return db
    .query("messages")
    .withSearchIndex("by_body", (q: any) =>
      q.search("body", "search query").eq("nonexistentField", "a"),
    )
    .collect();
});

export const missingSearchFilter = query(async ({ db }) => {
  return db
    .query("messages")
    .withSearchIndex("by_body", (q: any) => q)
    .collect();
});

export const tooManyFilterConditions = query(
  async ({ db }, { numFilterConditions }: { numFilterConditions: number }) => {
    return db
      .query("messages")
      .withSearchIndex("by_body", (q) => {
        let filter = q.search("body", "search query");
        for (let i = 0; i < numFilterConditions; i++) {
          filter = filter.eq("filterField", "filter value");
        }
        return filter;
      })
      .collect();
  },
);

export const insertMany = mutation(
  async (
    { db },
    {
      body,
      numDocumentsToCreate,
    }: { body: string; numDocumentsToCreate: number },
  ) => {
    for (let i = 0; i < numDocumentsToCreate; i++) {
      await db.insert("messages", {
        body,
      });
    }
  },
);
