import {
  internalAction,
  internalMutation,
  internalQuery,
} from "./_generated/server";
import { v } from "convex/values";
import { internal } from "./_generated/api";
import { Doc } from "./_generated/dataModel";
import { getLatestAgentSkillsSha } from "./util/agentSkills";

export const refresh = internalAction({
  args: {},
  handler: async (ctx): Promise<Doc<"agentSkills"> | null> => {
    try {
      const sha = await getLatestAgentSkillsSha();
      return await ctx.runMutation(internal.agentSkills.save, { sha });
    } catch (error) {
      console.error("Failed to refresh agent skills SHA:", error);
      return null;
    }
  },
});

export const getCached = internalQuery({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.query("agentSkills").first();
  },
});

export const save = internalMutation({
  args: { sha: v.string() },
  handler: async (ctx, { sha }) => {
    const existing = await ctx.db.query("agentSkills").collect();
    for (const entry of existing) {
      await ctx.db.delete(entry._id);
    }
    const doc = await ctx.db.insert("agentSkills", { sha });
    return (await ctx.db.get(doc))!;
  },
});
