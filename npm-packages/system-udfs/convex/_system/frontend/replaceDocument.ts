import { ConvexError, GenericId } from "convex/values";
import { v } from "convex/values";
import { mutationGeneric } from "convex/server";

export default mutationGeneric({
  args: { id: v.string(), document: v.any() },
  handler: async (
    { db },
    args,
  ): Promise<{ success: false; error: string } | { success: true }> => {
    const id = args.id as GenericId<string>;
    const document = args.document;
    const doc = await db.get(id);
    if (doc === null) {
      return {
        success: false,
        error: "Document does not exist.",
      };
    }

    try {
      await db.replace(id, document);
    } catch (e: any) {
      // Rewrapping this error because it could be a schema validation error.
      throw new ConvexError(e.message);
    }
    return { success: true };
  },
});
