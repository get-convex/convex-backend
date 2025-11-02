import { actionGeneric } from "convex/server";

export const demo = actionGeneric({
  handler: async (_ctx) => {
    return "old school";
  },
});
