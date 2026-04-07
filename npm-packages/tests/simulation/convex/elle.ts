import { v } from "convex/values";
import { mutation, query, QueryCtx } from "./_generated/server";
import { Id } from "./_generated/dataModel";

export const initializeRegister = mutation({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.insert("elleRegisters", {});
  },
});

export const getRegister = query({
  args: {
    id: v.id("elleRegisters"),
  },
  returns: v.array(v.number()),
  handler: async (ctx, args) => {
    return await loadRegister(ctx, args.id);
  },
});

export const appendRegister = mutation({
  args: {
    id: v.id("elleRegisters"),
    value: v.number(),
  },
  returns: v.array(v.number()),
  handler: async (ctx, args) => {
    const register = await loadRegister(ctx, args.id);
    const offset = register.length;
    await ctx.db.insert("elleRegisterValues", {
      registerId: args.id,
      offset,
      value: args.value,
    });
    const result = [...register, args.value];
    console.log("[appendRegister]", args, result);
    return result;
  },
});

async function loadRegister(ctx: QueryCtx, id: Id<"elleRegisters">) {
  const register = await ctx.db.get(id);
  if (!register) {
    throw new Error(`Invalid ID: ${id}`);
  }
  const documents = await ctx.db
    .query("elleRegisterValues")
    .withIndex("registerId", (q) => q.eq("registerId", id))
    .collect();
  const result = [];
  for (const document of documents) {
    if (document.offset !== result.length) {
      throw new Error(`Invalid offset: ${document.offset}`);
    }
    result.push(document.value);
  }
  return result;
}
