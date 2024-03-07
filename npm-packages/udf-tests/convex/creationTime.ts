import { query, mutation } from "./_generated/server";

export const createFiveDocuments = mutation(async ({ db }) => {
  for (let i = 0; i < 5; i++) {
    await db.insert("table", { count: i });
  }
});

export const getDocumentsByCreationTime = query(({ db }) => {
  return db.query("table").withIndex("by_creation_time").collect();
});
