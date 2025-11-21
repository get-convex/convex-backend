import { mutation } from "./_generated/server";

export const store = mutation({
  args: {},
  handler: async ({ db, auth }) => {
    const identity = await auth.getUserIdentity();
    if (!identity) {
      throw new Error("Called storeUser without authentication present");
    }

    // Check if we've already stored this identity before.
    const user = await db
      .query("users")
      .withIndex("by_token", (q) =>
        q.eq("tokenIdentifier", identity.tokenIdentifier),
      )
      .unique();
    if (user !== null) {
      // If we've seen this identity before but the name has changed, patch the value.
      if (user.name !== identity.name) {
        await db.patch(user._id, { name: identity.name });
      }
      return user._id;
    }
    // If it's a new identity, create a new `User`.
    return db.insert("users", {
      name: identity.name!,
      tokenIdentifier: identity.tokenIdentifier,
    });
  },
});
