import { query, mutation } from "./_generated/server";
import { Id } from "./_generated/dataModel";

export const generateUploadUrl = mutation({
  args: {},
  handler: async ({ storage }) => {
    return await storage.generateUploadUrl();
  },
});

export const getImageUrl = query({
  handler: async (
    { storage },
    { storageId }: { storageId: string | Id<"_storage"> },
  ) => {
    return await storage.getUrl(storageId);
  },
});

export const deleteById = mutation({
  handler: async (
    { storage },
    { storageId }: { storageId: string | Id<"_storage"> },
  ) => {
    return await storage.delete(storageId);
  },
});

export const getMetadata = query({
  handler: async (
    { storage },
    { storageId }: { storageId: string | Id<"_storage"> },
  ) => {
    return await storage.getMetadata(storageId);
  },
});

export const list = query({
  args: {},
  handler: async ({ db }) => {
    return await db.system.query("_storage").collect();
  },
});

export const get = query({
  handler: async ({ db }, { id }: { id: Id<"_storage"> }) => {
    return await db.system.get(id);
  },
});
