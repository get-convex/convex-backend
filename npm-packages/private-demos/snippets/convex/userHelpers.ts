// @snippet start userHelpers
import { Doc } from "./_generated/dataModel";
import { QueryCtx } from "./_generated/server";

export async function getCurrentUser(ctx: QueryCtx): Promise<Doc<"users">> {
  // load user details using `ctx.auth` and `ctx.db`
  // @snippet end userHelpers
  return null as any;
}
