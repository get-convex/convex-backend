import { query } from "./_generated/server";
import { v } from "convex/values";

export const eventAttendees = query({
  args: { eventId: v.id("events") },
  handler: async (ctx, args) => {
    const event = await ctx.db.get("events", args.eventId);
    // highlight-start
    return Promise.all(
      (event?.attendeeIds ?? []).map((userId) => ctx.db.get("users", userId)),
    );
    // highlight-end
  },
});
