"use node";
import { Id } from "./_generated/dataModel";
import { action } from "./_generated/server";

export const generateUploadUrl = action(async ({ storage }) => {
  return await storage.generateUploadUrl();
});

export const getUrl = action(
  async (
    { storage },
    { storageId }: { storageId: Id<"_storage"> | string },
  ) => {
    return await storage.getUrl(storageId);
  },
);

export const deleteById = action(
  async (
    { storage },
    { storageId }: { storageId: Id<"_storage"> | string },
  ) => {
    return await storage.delete(storageId);
  },
);

export const getMetadata = action(
  async (
    { storage },
    { storageId }: { storageId: Id<"_storage"> | string },
  ) => {
    return await storage.getMetadata(storageId);
  },
);

export const store = action(
  async (
    { storage },
    { content, contentType }: { content: any; contentType: string },
  ) => {
    const blob = new Blob([content], {
      type: contentType,
    });
    return await storage.store(blob);
  },
);

export const get = action(
  async (
    { storage },
    { storageId }: { storageId: Id<"_storage"> | string },
  ) => {
    const blob = await storage.get(storageId);
    return await blob?.text();
  },
);
