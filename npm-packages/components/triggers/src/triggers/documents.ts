import { FunctionHandle } from "convex/server";
import { mutation } from "./_generated/server.js";
import { v } from "convex/values";
import type { TriggerArgs } from "../types.js";

export const insert = mutation({
  args: {
    value: v.any(),
    atomicInsert: v.string(),
    triggers: v.array(v.string()),
  },
  returns: v.string(),
  handler: async (ctx, { value, atomicInsert, triggers }) => {
    const { newDoc } = await ctx.runMutation(
      atomicInsert as FunctionHandle<"mutation">,
      { value },
    );
    for (const trigger of triggers) {
      await ctx.runMutation(
        trigger as FunctionHandle<"mutation", TriggerArgs<any, any>>,
        {
          change: { type: "insert", id: newDoc._id, oldDoc: null, newDoc },
        },
      );
    }
    return newDoc._id;
  },
});

export const patch = mutation({
  args: {
    id: v.string(),
    value: v.any(),
    atomicPatch: v.string(),
    triggers: v.array(v.string()),
  },
  returns: v.null(),
  handler: async (ctx, { id, value, atomicPatch, triggers }) => {
    const { oldDoc, newDoc } = await ctx.runMutation(
      atomicPatch as FunctionHandle<"mutation">,
      { id, value },
    );
    for (const trigger of triggers) {
      await ctx.runMutation(
        trigger as FunctionHandle<"mutation", TriggerArgs<any, any>>,
        {
          change: { type: "patch", id: id as any, oldDoc, newDoc },
        },
      );
    }
  },
});

export const replace = mutation({
  args: {
    id: v.string(),
    value: v.any(),
    atomicReplace: v.string(),
    triggers: v.array(v.string()),
  },
  returns: v.null(),
  handler: async (ctx, { id, value, atomicReplace, triggers }) => {
    const { oldDoc, newDoc } = await ctx.runMutation(
      atomicReplace as FunctionHandle<"mutation">,
      { id, value },
    );
    for (const trigger of triggers) {
      await ctx.runMutation(
        trigger as FunctionHandle<"mutation", TriggerArgs<any, any>>,
        {
          change: { type: "replace", id: id as any, oldDoc, newDoc },
        },
      );
    }
  },
});

export const deleteDoc = mutation({
  args: {
    id: v.string(),
    atomicDelete: v.string(),
    triggers: v.array(v.string()),
  },
  returns: v.null(),
  handler: async (ctx, { id, atomicDelete, triggers }) => {
    const { oldDoc } = await ctx.runMutation(
      atomicDelete as FunctionHandle<"mutation">,
      { id },
    );
    for (const trigger of triggers) {
      await ctx.runMutation(
        trigger as FunctionHandle<"mutation", TriggerArgs<any, any>>,
        {
          change: { type: "delete", id: id as any, oldDoc, newDoc: null },
        },
      );
    }
  },
});
