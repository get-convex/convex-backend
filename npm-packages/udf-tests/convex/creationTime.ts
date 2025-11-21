import { query, mutation } from "./_generated/server";

export const createFiveDocuments = mutation({
  args: {},
  handler: async ({ db }) => {
    for (let i = 0; i < 5; i++) {
      await db.insert("table", { count: i });
    }
  },
});

export const getDocumentsByCreationTime = query({
  args: {},
  handler: ({ db }) => {
    return db.query("table").withIndex("by_creation_time").collect();
  },
});
