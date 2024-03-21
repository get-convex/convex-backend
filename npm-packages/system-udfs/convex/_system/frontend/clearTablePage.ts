import { Cursor } from "convex/server";
import { mutationGeneric } from "../server";
import { v } from "convex/values";

export const MAX_CLEAR_ROWS = 4000;

export default mutationGeneric({
  args: {
    tableName: v.string(),
    cursor: v.union(v.string(), v.null()),
  },
  handler: async (
    ctx,
    args,
  ): Promise<{
    deleted: number;
    hasMore: boolean;
    continueCursor: Cursor;
  }> => {
    const { db } = ctx;
    const { tableName, cursor } = args;
    // Delete from oldest to newest to avoid repeatedly invalidating the query
    // run by the data page
    const {
      page: documents,
      continueCursor,
      isDone,
    } = await db
      .query(tableName)
      .withIndex("by_creation_time")
      .order("asc")
      .paginate({
        numItems: MAX_CLEAR_ROWS,
        cursor,
        // We can read up to 8MiB, but we're currently double counting the docs
        // when they're read in this query and also when they're deleted. So make
        // this limit conservative to avoid hitting limits and crashing.
        maximumBytesRead: 4000000,
      });

    await Promise.all(documents.map((doc) => db.delete(doc._id)));

    return {
      deleted: documents.length,
      continueCursor,
      hasMore: !isDone,
    };
  },
});
