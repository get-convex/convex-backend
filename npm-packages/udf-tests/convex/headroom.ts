import { getConvexSize, getDocumentSize } from "convex/values";
import { getTransactionHeadroom, TransactionHeadroom } from "convex/server";
import { api } from "./_generated/api";
import { query, mutation, action } from "./_generated/server";

export const headroomEmpty = query({
  handler: async () => {
    return await getTransactionHeadroom();
  },
});

export const headroomAfterInsert = mutation({
  handler: async ({
    db,
  }): Promise<{ headroom: TransactionHeadroom; docSize: number }> => {
    const doc = { body: "hello", channel: "general" };
    const id = await db.insert("messages", doc);
    const inserted = await db.get(id);
    const docSize = getDocumentSize(inserted!);
    const headroom = await getTransactionHeadroom();
    return { headroom, docSize };
  },
});

export const headroomAfterQuery = query({
  handler: async ({
    db,
  }): Promise<{
    headroom: TransactionHeadroom;
    totalBytes: number;
    docCount: number;
  }> => {
    const docs = await db.query("messages").collect();
    const headroom = await getTransactionHeadroom();
    const totalBytes = docs.reduce((sum, doc) => sum + getDocumentSize(doc), 0);
    return { headroom, totalBytes, docCount: docs.length };
  },
});

export const headroomWithSubTransactions = mutation({
  handler: async (ctx): Promise<any> => {
    const initial = await getTransactionHeadroom();
    const insertResult = await ctx.runMutation(
      api.headroom.headroomAfterInsert,
      {},
    );
    const afterInsert = await getTransactionHeadroom();
    const emptyQuery = await ctx.runQuery(api.headroom.headroomEmpty, {});
    const afterEmptyQuery = await getTransactionHeadroom();
    const queryResult = await ctx.runQuery(api.headroom.headroomAfterQuery, {});
    const final_ = await getTransactionHeadroom();
    return {
      initial,
      insertResult,
      afterInsert,
      emptyQuery,
      afterEmptyQuery,
      queryResult,
      final_,
    };
  },
});

export const headroomAfterSystemRead = mutation({
  handler: async (ctx) => {
    const before = await getTransactionHeadroom();
    await ctx.db.system.query("_scheduled_functions").first();
    await ctx.db.system.query("_storage").first();
    const after = await getTransactionHeadroom();
    return { before, after };
  },
});

export const headroomAfterSchedule = mutation({
  handler: async (ctx) => {
    const before = await getTransactionHeadroom();
    const args = { body: "scheduled message", channel: "general" };
    const expectedArgSize = getConvexSize([args]);
    await ctx.scheduler.runAfter(1000, api.basic.insertObject, args);
    const afterOne = await getTransactionHeadroom();
    await ctx.scheduler.runAfter(1000, api.basic.insertObject, args);
    const afterTwo = await getTransactionHeadroom();
    return { before, afterOne, afterTwo, expectedArgSize };
  },
});

export const headroomExceedLimit = mutation({
  handler: async ({ db }) => {
    const bigString = "x".repeat(900_000);
    // Insert large documents until we exceed the bytesWritten limit.
    let count = 0;
    for (let i = 0; i < 20; i++) {
      try {
        await db.insert("messages", { body: bigString, channel: "general" });
        count++;
      } catch {
        break;
      }
    }
    const headroom = await getTransactionHeadroom();
    return { headroom, count };
  },
});

export const headroomFromAction = action({
  handler: async () => {
    return await getTransactionHeadroom();
  },
});
