import { defineTable, defineSchema } from "convex/server";
import { v } from "convex/values";

export const primitiveTypesByName = {
  str: v.string(),
  literal: v.literal("literal"),
  num: v.number(),
  bool: v.boolean(),
  data: v.bytes(),
  arr: v.array(v.number()),
  null: v.null(),
  id: v.id("empty"),
};
export const primitiveTypes = Object.values(primitiveTypesByName);
export const validators = [
  v.null(),
  v.id("empty"),
  v.string(),
  v.number(),
  v.boolean(),
  v.bytes(),
  v.array(v.number()),
  v.object(primitiveTypesByName),
  v.record(v.string(), v.null()),
] as const;

export default defineSchema({
  empty: defineTable({}),
  primitiveTypes: defineTable(primitiveTypesByName)
    .index("str", ["str"])
    .index("literal", ["literal"])
    .index("num", ["num"])
    .index("bool", ["bool"])
    .index("data", ["data"])
    .index("arr", ["arr"])
    .index("null", ["null"])
    .index("id", ["id"])
    .searchIndex("search_str", {
      searchField: "str",
      filterFields: ["literal", "num", "bool", "data", "arr", "null", "id"],
    })
    .vectorIndex("vector_arr", {
      vectorField: "arr",
      dimensions: 10,
      filterFields: ["literal", "num", "bool", "data", "null", "id"],
    }),
  objectTypes: defineTable({
    obj: v.object(primitiveTypesByName),
    optional: v.optional(v.object(primitiveTypesByName)),
    parent: v.object({
      child: v.object(primitiveTypesByName),
    }),
  })
    // Index on nested field
    .index("parent", ["parent"])
    .index("child", ["parent.child"])
    .index("str", ["parent.child.str"])
    .index("num_bool", ["parent.child.num", "parent.child.bool"]),
  topLevelUnion: defineTable(
    v.union(v.object({}), v.object(primitiveTypesByName)),
  ).index("bool", ["bool"]),
  unionTypes: defineTable({
    union: v.union(...validators),
    literals: v.union(v.literal("literal1"), v.literal("literal2")),
    optional: v.optional(v.union(...validators)),
  })
    .index("union_optional", ["union", "optional"])
    .searchIndex("union", {
      searchField: "union",
      filterFields: ["optional"],
    })
    .vectorIndex("vector_union", {
      vectorField: "union",
      dimensions: 100,
      filterFields: ["optional"],
    }),
  recordTypes: defineTable({
    strKey: v.record(v.string(), v.object(primitiveTypesByName)),
    idKey: v.record(v.id("empty"), v.union(v.null(), v.string())),
  })
    .index("strKey_idKey", ["strKey", "idKey"])
    .searchIndex("strKey", {
      searchField: "strKey",
      filterFields: ["idKey"],
    }),
  messages: defineTable({
    author: v.string(),
    body: v.string(),
  }),
  notes: defineTable({
    text: v.string(),
  }),
});
