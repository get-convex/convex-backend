import { v } from "convex/values";
import { mutation } from "./_generated/server";

export const importEmbedding = mutation({
  args: {
    docs: v.array(
      v.object({
        input: v.string(),
        embedding: v.array(v.float64()),
      }),
    ),
  },
  handler: async (ctx, args) => {
    const promises = [];
    for (const doc of args.docs) {
      promises.push(ctx.db.insert("embeddings", doc));
    }
    await Promise.all(promises);
  },
});
