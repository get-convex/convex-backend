import { describe, expect, test } from "vitest";
import {
  inlineMutationToMutationSource,
  inlineQueryToQuerySource,
} from "./runTestFunction.js";

describe("runTestFunction helpers", () => {
  test("wraps inline queries with the query preamble and implicit return", () => {
    expect(inlineQueryToQuerySource('await ctx.db.query("messages").take(5)'))
      .toBe(`import { query, internalQuery } from "convex:/_system/repl/wrappers.js";

export default query({
  handler: async (ctx) => {
    return (await ctx.db.query("messages").take(5));
  },
});`);
  });

  test("wraps inline mutations with the mutation preamble", () => {
    expect(
      inlineMutationToMutationSource(
        'const id = await ctx.db.insert("messages", { body: "hello" });\nreturn { id };',
      ),
    )
      .toBe(`import { mutation, internalMutation } from "convex:/_system/repl/wrappers.js";

export default mutation({
  handler: async (ctx) => {
    const id = await ctx.db.insert("messages", { body: "hello" });
    return { id };
  },
});`);
  });

  test("does not duplicate the wrappers import for mutation modules", () => {
    const source = `import { mutation, internalMutation } from "convex:/_system/repl/wrappers.js";

export default mutation({
  handler: async (ctx) => {
    return await ctx.db.insert("messages", { body: "hello" });
  },
});`;

    expect(inlineMutationToMutationSource(source)).toBe(source);
  });
});
