import { query } from "./_generated/server";

export const doNothing = query({
  handler: async (_ctx, _args: { x: any }) => {},
});
