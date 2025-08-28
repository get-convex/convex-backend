import { v } from "convex/values";
import {
  internalAction,
  internalMutation,
  internalQuery,
} from "./_generated/server";
import { internal } from "./_generated/api";
import { type } from "arktype";
import { Doc } from "./_generated/dataModel";

export const refresh = internalAction({
  args: {},
  handler: async (ctx): Promise<Doc<"npmVersion"> | null> => {
    try {
      const response = await fetch("https://registry.npmjs.org/convex/latest");
      if (!response.ok) {
        throw new Error(`Failed to fetch NPM data: ${response.status}`);
      }

      const NpmResponse = type({
        version: "string",
      });

      const out = NpmResponse(await response.json());
      if (out instanceof type.errors) {
        throw new Error("Invalid NPM response: " + out.summary);
      }

      return await ctx.runMutation(internal.npm.save, {
        version: out.version,
      });
    } catch (error) {
      console.error("Failed to refresh NPM version:", error);
      return null;
    }
  },
});

export const getCached = internalQuery({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.query("npmVersion").first();
  },
});

export const save = internalMutation({
  args: { version: v.string() },
  handler: async (ctx, args) => {
    // Delete existing entries
    const existing = await ctx.db.query("npmVersion").collect();
    for (const entry of existing) {
      await ctx.db.delete(entry._id);
    }

    // Insert new entry
    const doc = await ctx.db.insert("npmVersion", { value: args.version });
    return (await ctx.db.get(doc))!;
  },
});
