import { query } from "./_generated/server";

export const authInfo = query(async (ctx) => {
  const userInfo = await ctx.auth.getUserIdentity();
  return userInfo;
});
