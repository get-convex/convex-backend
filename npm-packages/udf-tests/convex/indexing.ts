import {
  MutationBuilder,
  mutationGeneric,
  QueryBuilder,
  queryGeneric,
} from "convex/server";
import {
  DataModelFromSchemaDefinition,
  defineSchema,
  defineTable,
} from "convex/server";
import { v } from "convex/values";

const _schema = defineSchema({
  myTable: defineTable({
    a: v.number(),
    b: v.optional(v.number()),
  }).index("by_a_b", ["a", "b"]),
});
type DataModel = DataModelFromSchemaDefinition<typeof _schema>;
const query: QueryBuilder<DataModel, "public"> = queryGeneric;
const mutation: MutationBuilder<DataModel, "public"> = mutationGeneric;

export const insert = mutation({
  handler: async ({ db }, { a, b }: { a: number; b: number }) => {
    await db.insert("myTable", {
      a,
      b,
    });
  },
});

export const insertMissingField = mutation({
  handler: async ({ db }, { a }: { a: number }) => {
    await db.insert("myTable", {
      a,
    });
  },
});

export const allItemsInIndex = query({
  args: {},
  handler: ({ db }) => {
    return db.query("myTable").withIndex("by_a_b").collect();
  },
});

export const oneFieldEquality = query({
  handler: ({ db }, { a }: any) => {
    return db
      .query("myTable")
      .withIndex("by_a_b", (q) => q.eq("a", a))
      .collect();
  },
});

export const twoFieldEquality = query({
  handler: ({ db }, { a, b }: any) => {
    return db
      .query("myTable")
      .withIndex("by_a_b", (q) => q.eq("a", a).eq("b", b))
      .collect();
  },
});

export const twoFieldEqualityExplicitMissing = query({
  handler: ({ db }, { a }: any) => {
    return db
      .query("myTable")
      .withIndex("by_a_b", (q) => q.eq("a", a).eq("b", undefined))
      .collect();
  },
});

export const twoFieldFilterEquality = query({
  handler: ({ db }, { a, b }: any) => {
    return db
      .query("myTable")
      .filter((q) => q.and(q.eq(q.field("a"), a), q.eq(q.field("b"), b)))
      .collect();
  },
});

export const twoFieldFilterEqualityExplicitMissing = query({
  handler: ({ db }, { a }: any) => {
    return db
      .query("myTable")
      .filter((q) =>
        q.and(q.eq(q.field("a"), a), q.eq(q.field("b"), undefined)),
      )
      .collect();
  },
});

export const twoFieldEqualityOutOfOrder = query({
  handler: ({ db }, { a, b }: any) => {
    return (
      db
        .query("myTable")
        // @ts-expect-error Intentional invalid syntax
        .withIndex("by_a_b", (q) => q.eq("b", b).eq("a", a))
        .collect()
    );
  },
});

export const exclusiveRangeOnFirstField = query({
  handler: ({ db }, { aStart, aEnd }: any) => {
    return db
      .query("myTable")
      .withIndex("by_a_b", (q) => q.gt("a", aStart).lt("a", aEnd))
      .collect();
  },
});

export const inclusiveRangeOnFirstField = query({
  handler: ({ db }, { aStart, aEnd }: any) => {
    return db
      .query("myTable")
      .withIndex("by_a_b", (q) => q.gte("a", aStart).lte("a", aEnd))
      .collect();
  },
});

export const exclusiveRangeOnSecondField = query({
  handler: ({ db }, { a, bStart, bEnd }: any) => {
    return db
      .query("myTable")
      .withIndex("by_a_b", (q) => q.eq("a", a).gt("b", bStart).lt("b", bEnd))
      .collect();
  },
});

export const inclusiveRangeOnSecondField = query({
  handler: ({ db }, { a, bStart, bEnd }: any) => {
    return db
      .query("myTable")
      .withIndex("by_a_b", (q) => q.eq("a", a).gte("b", bStart).lte("b", bEnd))
      .collect();
  },
});

export const rangeOnSecondFieldOutOfOrder = query({
  handler: ({ db }, { a, bStart, bEnd }: any) => {
    return db
      .query("myTable")
      .withIndex("by_a_b", (q) =>
        // @ts-expect-error Intentional invalid syntax
        q.gte("b", bStart).eq("a", a).lte("b", bEnd),
      )
      .collect();
  },
});

export const rangeFirstFieldGtUndefined = query({
  args: {},
  handler: ({ db }) => {
    return db
      .query("myTable")
      .withIndex("by_a_b", (q) => q.gt("a", undefined as any))
      .collect();
  },
});

export const rangeSecondFieldGtUndefined = query({
  handler: ({ db }, { a }: { a: number }) => {
    return db
      .query("myTable")
      .withIndex("by_a_b", (q) => q.eq("a", a).gt("b", undefined))
      .collect();
  },
});

/**
 * Error cases.
 */

export const invalidIndexRange = query({
  args: {},
  handler: ({ db }) => {
    // The index is on ("a", "b") but the index range starts with "b".
    return (
      db
        .query("myTable")
        // @ts-expect-error Intentional invalid syntax
        .withIndex("by_a_b", (q) => q.eq("b", 1))
        .collect()
    );
  },
});

export const eqFieldNotInIndex = query({
  args: {},
  handler: ({ db }) => {
    // The index is on ("a", "b") but the index range starts with "b".
    return (
      db
        .query("myTable")
        // @ts-expect-error Intentional invalid syntax
        .withIndex("by_a_b", (q) => q.eq("c", 1))
        .collect()
    );
  },
});

export const ltFieldNotInIndex = query({
  args: {},
  handler: ({ db }) => {
    // The index is on ("a", "b") but the index range starts with "b".
    return (
      db
        .query("myTable")
        // @ts-expect-error Intentional invalid syntax
        .withIndex("by_a_b", (q) => q.lt("c", 1))
        .collect()
    );
  },
});

export const defineBoundsTwice = query({
  args: {},
  handler: ({ db }) => {
    // Two lower bounds isn't valid.
    return (
      db
        .query("myTable")
        // @ts-expect-error Intentional invalid syntax
        .withIndex("by_a_b", (q) => q.gt("a", 2).gte("a", 1))
        .collect()
    );
  },
});

export const defineEqualityBoundsTwice = query({
  args: {},
  handler: ({ db }) => {
    // Two equality bounds on the same field isn't valid.
    return (
      db
        .query("myTable")
        // @ts-expect-error Intentional invalid syntax
        .withIndex("by_a_b", (q) => q.eq("a", 2).eq("a", 1))
        .collect()
    );
  },
});

export const equalityAndInequalityOverlap = query({
  args: {},
  handler: ({ db }) => {
    // An equality and inequality bound on the same field isn't valid.
    return (
      db
        .query("myTable")
        // @ts-expect-error Intentional invalid syntax
        .withIndex("by_a_b", (q) => q.eq("a", 2).gt("a", 1))
        .collect()
    );
  },
});

export const boundsOnDifferentFields = query({
  args: {},
  handler: ({ db }) => {
    // Can't have bounds on both "a" and "b".
    return (
      db
        .query("myTable")
        // @ts-expect-error Intentional invalid syntax
        .withIndex("by_a_b", (q) => q.gt("a", 2).lt("b", 1))
        .collect()
    );
  },
});
