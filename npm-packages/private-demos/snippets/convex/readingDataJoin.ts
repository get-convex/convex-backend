import { query } from "./_generated/server";
import { v } from "convex/values";

export const eventAttendees = query({
  args: { eventId: v.id("events") },
  handler: async (ctx, args) => {
    const event = await ctx.db.get("events", args.eventId);
    if (!event) return [];

    // highlight-start
    // Join on attendees via an index lookup
    const attendees = await ctx.db
      .query("attendees")
      .withIndex("by_eventId", (q) => q.eq("eventId", event._id))
      .take(10_000); // Safeguard: only return the first 10k users
    // Join attendees on users via a direct lookup
    return Promise.all(
      attendees.map(async (attendee) => {
        const user = await ctx.db.get("users", attendee.userId);
        return {
          name: user?.name,
          userId: user?._id,
          attendeeId: attendee._id,
        };
      }),
    );
    // highlight-end
  },
});
