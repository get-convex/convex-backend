import { action, internalMutation, query } from "./_generated/server";
import { internal } from "./_generated/api";
import { v } from "convex/values";

export const generateAndStore = action({
  args: { prompt: v.string() },
  handler: async (ctx, args) => {
    // Not shown: generate imageUrl from `prompt`
    const imageUrl = "https://....";

    // Download the image
    const response = await fetch(imageUrl);
    const image = await response.blob();

    // Store the image in Convex
    const storageId = await ctx.storage.store(image);

    // Write `storageId` to a document
    await ctx.runMutation(internal.images.storeResult, {
      storageId,
      prompt: args.prompt,
    });
  },
});

export const storeResult = internalMutation({
  args: {
    storageId: v.id("_storage"),
    prompt: v.string(),
  },
  handler: async (ctx, args) => {
    const { storageId, prompt } = args;
    await ctx.db.insert("images", { storageId, prompt });
  },
});
