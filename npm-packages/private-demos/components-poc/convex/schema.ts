import { defineSchema, defineTable, FunctionHandle } from "convex/server";
import { v } from "convex/values";
import { functionValidator } from "./types";

export default defineSchema({
  messages: defineTable({
    author: v.string(),
    body: v.string(),
  }),
  functionHandles: defineTable({
    untyped: functionValidator<FunctionHandle<"query">>(),
    typed:
      functionValidator<
        FunctionHandle<"query", { a: number; b: number }, number>
      >(),
  }),

  notes: defineTable({
    text: v.string(),
  }),
});
