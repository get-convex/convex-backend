import { PaginationResult, SystemDataModel } from "convex/server";
import { mutationGeneric } from "../../server";
import { Id } from "../../_generated/dataModel";
import { v } from "convex/values";
import { paginationOptsValidator } from "convex/server";
import { queryGeneric } from "../secretSystemTables";

export const numFiles = queryGeneric({
  args: {},
  handler: async ({ db }): Promise<number> => {
    return await db.system.query("_storage").count();
  },
});

export type FileMetadata = SystemDataModel["_storage"]["document"] & {
  url: string;
};

export const fileMetadata = queryGeneric({
  args: { paginationOpts: paginationOptsValidator },
  handler: async (
    { db, storage },
    { paginationOpts },
  ): Promise<PaginationResult<FileMetadata>> => {
    const files = await db.system
      .query("_storage")
      .order("desc")
      .paginate(paginationOpts);

    const newPage = await Promise.all(
      files.page.map(async (file) => {
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        const url = (await storage.getUrl(file._id))!;
        return {
          url,
          ...file,
        };
      }),
    );
    return {
      ...files,
      page: newPage,
    };
  },
});

export const deleteFile = mutationGeneric(
  async (
    { storage },
    { storageId }: { storageId: Id<"_storage"> },
  ): Promise<void> => {
    return await storage.delete(storageId);
  },
);

export const deleteFiles = mutationGeneric({
  args: {
    storageIds: v.array(v.id("_storage")),
  },
  handler: async ({ storage }, { storageIds }): Promise<void> => {
    for (const storageId of storageIds) {
      await storage.delete(storageId);
    }
  },
});

export const generateUploadUrl = mutationGeneric(
  async ({ storage }): Promise<string> => {
    return await storage.generateUploadUrl();
  },
);
