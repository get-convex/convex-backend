import { GenericId, v } from "convex/values";
import { query, mutation } from "./_generated/server";

export const cancelMessage = mutation({
  args: { jobId: v.id("_scheduled_functions") },
  handler: async (ctx, args) => {
    await ctx.scheduler.cancel(args.jobId);
  },
});

export const listScheduledSends = query({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.system.query("_scheduled_functions").collect();
  },
});

export const listFiles = query({
  args: {},
  handler: async (ctx) => {
    const fileUploads = await ctx.db.query("file_uploads").collect();
    const results: {
      _id: GenericId<"_storage">;
      _creationTime: number;
      contentType?: string | undefined;
      sha256: string;
      size: number;
      author: string;
    }[] = [];
    for (const fileUpload of fileUploads) {
      const fileMetadata = await ctx.db.system.get(fileUpload.upload_id);

      if (fileMetadata !== null) {
        results.push({
          ...fileMetadata,
          author: fileUpload.author,
        });
      }
    }

    return results;
  },
});

export const getFileUrl = query({
  args: { id: v.id("_storage") },
  handler: async (ctx, args) => {
    return await ctx.storage.getUrl(args.id);
  },
});
