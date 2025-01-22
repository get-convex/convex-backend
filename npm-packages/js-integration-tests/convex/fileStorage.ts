import { query, mutation } from "./_generated/server";
import { Id } from "./_generated/dataModel";

export const generateUploadUrl = mutation(async ({ storage }) => {
  return await storage.generateUploadUrl();
});

export const getImageUrl = query(
  async (
    { storage },
    { storageId }: { storageId: string | Id<"_storage"> },
  ) => {
    return await storage.getUrl(storageId);
  },
);

export const deleteById = mutation(
  async (
    { storage },
    { storageId }: { storageId: string | Id<"_storage"> },
  ) => {
    return await storage.delete(storageId);
  },
);

export const getMetadata = query(
  async (
    { storage },
    { storageId }: { storageId: string | Id<"_storage"> },
  ) => {
    return await storage.getMetadata(storageId);
  },
);

export const list = query({
  handler: async ({ db }) => {
    return await db.system.query("_storage").collect();
  },
});

export const get = query({
  handler: async ({ db }, { id }: { id: Id<"_storage"> }) => {
    return await db.system.get(id);
  },
});
