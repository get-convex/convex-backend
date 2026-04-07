import { QueryCtx } from "./_generated/server";

// TODO: Implement auth in test framework.
export async function getCurrentUserIdOrThrow(ctx: QueryCtx) {
  const user = await ctx.db.query("users").first();
  if (user === null) {
    throw new Error("No users in database");
  }
  return user._id;
}
