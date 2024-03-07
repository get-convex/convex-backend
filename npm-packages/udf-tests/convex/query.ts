import { PaginationOptions, paginationOptsValidator } from "convex/server";
import { Id } from "./_generated/dataModel";
import { mutation, query } from "./_generated/server";

export const insert = mutation(function (
  { db },
  { number }: { number: number },
) {
  return db.insert("test", { hello: number });
});

export const deleteDoc = mutation(function (
  { db },
  { id }: { id: Id<"test"> },
) {
  return db.delete(id);
});

export const get = query(function ({ db }, { id }: { id: Id<"test"> }) {
  return db.get(id);
});

export const filterScan = query(({ db }, { number }: { number: number }) =>
  db
    .query("test")
    .filter((q) => q.eq(q.field("hello"), number))
    .collect(),
);

export const explicitScan = query(({ db }, { number }: { number: number }) => {
  return db
    .query("test")
    .fullTableScan()
    .filter((q) => q.eq(q.field("hello"), number))
    .collect();
});

/**
 * Boolean value filters
 *
 * These are kinda silly because `.filter(_q => true)` includes
 * everything and `.filter(_q => false)` filters everything out.
 * This functionality is useful though when programatically creating a filter
 * expression that might be a noop.
 */

export const trueLiteralFilter = query(({ db }) =>
  db
    .query("test")
    .filter((_q) => true)
    .collect(),
);
export const falseLiteralFilter = query(({ db }) =>
  db
    .query("test")
    .filter((_q) => false)
    .collect(),
);

export const paginateTableScan = query({
  args: { paginationOpts: paginationOptsValidator },
  handler: async ({ db }, { paginationOpts }) => {
    return await db.query("test").paginate(paginationOpts);
  },
});

export const paginateIndex = query({
  args: { paginationOpts: paginationOptsValidator },
  handler: async ({ db }, { paginationOpts }) => {
    return await db
      .query("test")
      .withIndex("by_hello")
      .paginate(paginationOpts);
  },
});

export const paginateWithOpts = query({
  args: { paginationOpts: paginationOptsValidator },
  handler: async (
    { db },
    { paginationOpts }: { paginationOpts: PaginationOptions },
  ) => {
    return await db.query("test").paginate(paginationOpts);
  },
});

export const paginateFilterTableScan = query(
  async ({ db }, { id, cursor }: { id: Id<"test">; cursor: string }) => {
    return await db
      .query("test")
      .filter((q) => q.eq(q.field("_id"), id))
      .paginate({ cursor, numItems: 1 });
  },
);

export const paginateReverseTableScan = query({
  args: { paginationOpts: paginationOptsValidator },
  handler: async ({ db }, { paginationOpts }) => {
    return await db.query("test").order("desc").paginate(paginationOpts);
  },
});

export const multiplePaginatedQueries = query(async ({ db }) => {
  await db.query("test").paginate({ cursor: null, numItems: 1 });
  await db.query("test").paginate({ cursor: null, numItems: 1 });
});

export const orderFilter = query(async ({ db }, { min }: { min: any }) => {
  return await db
    .query("test")
    .order("desc")
    .filter((q) => q.gte(q.field("hello"), min))
    .collect();
});

export const filterOrder = query(async ({ db }, { min }: { min: any }) => {
  return await db
    .query("test")
    .filter((q) => q.gte(q.field("hello"), min))
    .order("desc")
    .collect();
});

export const orderOrder = query(async ({ db }) => {
  // Typescript does not let you do .order().order(), but if you do it
  // in JS, it should fail at runtime.
  const q: any = db.query("test").order("desc");
  return q.order("desc").collect();
});
