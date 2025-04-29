import { v } from "convex/values";
import { mutation, query } from "./_generated/server";
import { api } from "./_generated/api";
import { assert } from "chai";
import { Id } from "./_generated/dataModel";
import { performOp } from "udf-syscall-ffi";

/**
 * Copied and pasted from dashboard/convex/_system/frontend,
 * so that it can be tested in UDF tests.
 */
export const getTableMapping = query(async () => {
  return performOp("getTableMapping");
});

export const normalizeId = query({
  args: { id: v.string(), table: v.string() },
  handler: ({ db }, { id, table }) => {
    const normalized = db.normalizeId(table as any, id);
    return { normalized };
  },
});

export const normalizeSystemId = query({
  args: { id: v.string(), table: v.string() },
  handler: ({ db }, { id, table }) => {
    const normalized = db.system.normalizeId(table as any, id);
    return normalized;
  },
});

export const schedule = mutation({
  args: {},
  handler: async ({ scheduler }): Promise<Id<"_scheduled_functions">> => {
    return await scheduler.runAfter(10000, api.idStrings.schedule, {});
  },
});

export const queryVirtualId = query({
  args: { id: v.id("_scheduled_functions") },
  handler: async ({ db }, { id }) => {
    // First fetch by ID with db.system.get.
    const doc = await db.system.get(id);
    assert(doc !== null);
    const creationTime = doc._creationTime;
    // Fetch by creation time and ID.
    const doc1 = await db.system
      .query("_scheduled_functions")
      .withIndex("by_creation_time", (q) =>
        (q.eq("_creationTime", creationTime) as any).eq("_id", id),
      )
      .unique();
    assert.deepEqual(doc1, doc);
    // Fetch with by_id.
    const doc2 = await db.system
      .query("_scheduled_functions")
      .withIndex("by_id", (q) => q.eq("_id", id))
      .unique();
    assert.deepEqual(doc2, doc);
    // Fetch with inequalities on ID.
    const doc3 = await db.system
      .query("_scheduled_functions")
      .withIndex("by_id", (q) => q.lte("_id", id))
      .unique();
    assert.deepEqual(doc3, doc);
    const doc4 = await db.system
      .query("_scheduled_functions")
      .withIndex("by_id", (q) => q.lt("_id", id))
      .unique();
    assert.strictEqual(doc4, null);
    const doc5 = await db.system
      .query("_scheduled_functions")
      .withIndex("by_id", (q) =>
        q.gt("_id", id.slice(0, -1) as Id<"_scheduled_functions">),
      )
      .unique();
    assert.deepEqual(doc5, doc);
    const doc6 = await db.system
      .query("_scheduled_functions")
      .withIndex("by_id", (q) =>
        q.gt("_id", (id + " ") as Id<"_scheduled_functions">),
      )
      .unique();
    assert.strictEqual(doc6, null);
  },
});
