import { mutation } from "./_generated/server";
import { v } from "convex/values";
import { getCurrentUser } from "./userHelpers";

export const remove = mutation({
  args: { teamId: v.id("teams") },
  handler: async (ctx, { teamId }) => {
    const currentUser = await getCurrentUser(ctx);
    await ensureTeamAdmin(ctx, currentUser, teamId);
    await ctx.db.delete(teamId);
  },
});

async function ensureTeamAdmin(ctx, user, teamId) {
  // use `ctx.db` to check that `user` is a team admin and throw an error otherwise
}
