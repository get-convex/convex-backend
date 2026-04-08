import { getConvexSize, getDocumentSize } from "convex/values";
import { TransactionMetrics } from "convex/server";
import { api } from "./_generated/api";
import { query, mutation } from "./_generated/server";

export const metricsEmptyQuery = query({
  handler: async (ctx) => {
    return await ctx.meta.getTransactionMetrics();
  },
});

export const metricsAfterInsert = mutation({
  handler: async (
    ctx,
  ): Promise<{ metrics: TransactionMetrics; docSize: number }> => {
    const doc = { body: "hello", channel: "general" };
    const id = await ctx.db.insert("messages", doc);
    const inserted = await ctx.db.get(id);
    const docSize = getDocumentSize(inserted!);
    const metrics = await ctx.meta.getTransactionMetrics();
    return { metrics, docSize };
  },
});

export const metricsAfterQuery = query({
  handler: async (
    ctx,
  ): Promise<{
    metrics: TransactionMetrics;
    totalBytes: number;
    docCount: number;
  }> => {
    const docs = await ctx.db.query("messages").collect();
    const metrics = await ctx.meta.getTransactionMetrics();
    const totalBytes = docs.reduce((sum, doc) => sum + getDocumentSize(doc), 0);
    return { metrics, totalBytes, docCount: docs.length };
  },
});

export const metricsWithSubTransactions = mutation({
  handler: async (ctx): Promise<any> => {
    const initial = await ctx.meta.getTransactionMetrics();
    const insertResult = await ctx.runMutation(
      api.transactionMetrics.metricsAfterInsert,
      {},
    );
    const afterInsert = await ctx.meta.getTransactionMetrics();
    const emptyQuery = await ctx.runQuery(
      api.transactionMetrics.metricsEmptyQuery,
      {},
    );
    const afterEmptyQuery = await ctx.meta.getTransactionMetrics();
    const queryResult = await ctx.runQuery(
      api.transactionMetrics.metricsAfterQuery,
      {},
    );
    const final_ = await ctx.meta.getTransactionMetrics();
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

export const metricsAfterSystemRead = mutation({
  handler: async (ctx) => {
    const before = await ctx.meta.getTransactionMetrics();
    await ctx.db.system.query("_scheduled_functions").first();
    await ctx.db.system.query("_storage").first();
    const after = await ctx.meta.getTransactionMetrics();
    return { before, after };
  },
});

export const metricsAfterSchedule = mutation({
  handler: async (ctx) => {
    const before = await ctx.meta.getTransactionMetrics();
    const args = { body: "scheduled message", channel: "general" };
    const expectedArgSize = getConvexSize([args]);
    await ctx.scheduler.runAfter(1000, api.basic.insertObject, args);
    const afterOne = await ctx.meta.getTransactionMetrics();
    await ctx.scheduler.runAfter(1000, api.basic.insertObject, args);
    const afterTwo = await ctx.meta.getTransactionMetrics();
    return { before, afterOne, afterTwo, expectedArgSize };
  },
});

export const metricsExceedLimit = mutation({
  handler: async (ctx) => {
    const bigString = "x".repeat(900_000);
    // Insert large documents until we exceed the bytesWritten limit.
    let count = 0;
    for (let i = 0; i < 20; i++) {
      try {
        await ctx.db.insert("messages", {
          body: bigString,
          channel: "general",
        });
        count++;
      } catch {
        break;
      }
    }
    const headroom = await ctx.meta.getTransactionMetrics();
    return { headroom, count };
  },
});
