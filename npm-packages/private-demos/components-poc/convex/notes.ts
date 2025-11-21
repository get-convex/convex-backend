import { v } from "convex/values";
import { query, internalMutation } from "./_generated/server";
import { atomicMutators, triggerArgsValidator } from "@convex-dev/triggers";
import { mutationWithTriggers } from "./triggers";

export const { atomicInsert, atomicPatch, atomicReplace, atomicDelete } =
  atomicMutators("notes");

export const list = query({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.query("notes").collect();
  },
});

export const onNote = internalMutation({
  args: triggerArgsValidator("notes"),
  handler: async (ctx, args) => {
    console.log("NOTE CHANGED", args);
  },
});

export const insert = mutationWithTriggers({
  args: { text: v.string() },
  handler: async (ctx, { text }) => {
    return await ctx.db.insert("notes", { text });
  },
});
