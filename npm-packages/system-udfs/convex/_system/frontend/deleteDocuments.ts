import { GenericId, v } from "convex/values";
import { mutationGeneric } from "../server";

const MAX_DOCUMENT_DELETIONS = 4096;

export default mutationGeneric({
  args: {
    componentId: v.optional(v.union(v.string(), v.null())),
    toDelete: v.array(v.object({ id: v.string(), tableName: v.string() })),
  },
  handler: async (
    { db },
    { toDelete },
  ): Promise<{ success: false; error: string } | { success: true }> => {
    if (toDelete.length > MAX_DOCUMENT_DELETIONS) {
      return {
        success: false,
        error: `You can't delete more than ${MAX_DOCUMENT_DELETIONS}. Try selecting fewer documents instead..`,
      };
    }
    for (const d of toDelete) {
      await db.delete(d.id as GenericId<string>);
    }
    return { success: true };
  },
});
