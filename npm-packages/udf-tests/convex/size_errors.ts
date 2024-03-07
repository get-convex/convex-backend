import { api } from "./_generated/api";
import { action, mutation, query } from "./_generated/server";

export const queryThrowsNestingError = query(() => {
  let result: any = 1;
  // 65 levels is too many
  for (let i = 0; i < 65; i++) {
    result = [result];
  }
  return result;
});

export const queryDoesNotThrowNestingError = query(() => {
  let result: any = 1;
  // 64 levels is ok
  for (let i = 0; i < 64; i++) {
    result = [result];
  }
  return result;
});

export const writeToNowhere = mutation(async (_, _args: any) => {});

export const actionThrowsArgumentNestingError = action(async (ctx) => {
  let nested: any = 1;
  // 64 levels plus 1 for arguments object is too many
  for (let i = 0; i < 64; i++) {
    nested = [nested];
  }
  await ctx.runMutation(api.size_errors.writeToNowhere, { x: nested });
});
