import { v } from "convex/values";
import { action, mutation, query } from "./_generated/server";
import { api } from "./_generated/api";
import { EXAMPLE_DATA } from "../foodData";

export const populate = action({
  args: {},
  handler: async (ctx) => {
    for (const doc of EXAMPLE_DATA) {
      await ctx.runMutation(api.foods.insertRow, {
        theLetterA: doc.theLetterA,
        cuisine: doc.cuisine,
        bOrC: doc.bOrC,
        description: doc.description,
        embedding: doc.embedding,
      });
    }
  },
});

export const insertRow = mutation({
  args: {
    description: v.string(),
    theLetterA: v.string(),
    cuisine: v.string(),
    bOrC: v.string(),
    embedding: v.array(v.float64()),
  },
  handler: async (ctx, args) => {
    await ctx.db.insert("foods", args);
  },
});

export const queryDocs = query({
  args: {
    ids: v.array(v.id("foods")),
  },
  handler: async (ctx, args) => {
    const result = [];
    for (const id of args.ids) {
      const current = await ctx.db.get(id);
      if (current) {
        result.push(current);
      }
    }
    return result;
  },
});
