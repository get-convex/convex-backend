import { v } from "convex/values";
import { query } from "./_generated/server";

// Will be a bit over for serialization and the array.
export const largeArray = query({
  args: { count: v.number(), character: v.string() },
  handler: async (_ctx, args) => {
    let left = args.count;
    const items = [];
    const largeString = "x".repeat(1_000_000);
    while (left > 0) {
      const item = largeString.slice(0, left);
      items.push(item);
      left = left - item.length;
    }
    return items;
  },
});
