import { query } from "./_generated/server";

export const myQuery = query({
  args: {},
  handler: async () => {
    const mod = await import("./helper.js");
    return mod.default();
  },
});
