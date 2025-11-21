import { query } from "./_generated/server";

export const addOneInt = query({
  handler: async (_, { x }) => {
    return x + 1n;
  },
});
