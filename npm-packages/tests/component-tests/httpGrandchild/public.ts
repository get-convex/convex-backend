import { query } from "./_generated/server";

export const greeting = query({
  args: {},
  handler: async () => {
    return "hello from grandchild query";
  },
});
