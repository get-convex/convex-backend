"use node";
import { v } from "convex/values";
import { action } from "./_generated/server";
// import sharp from 'sharp';

export const getUploadUrl = action({
  args: {},
  handler: async (ctx) => {
    return await ctx.storage.generateUploadUrl();
  },
});

export const l = action({
  args: {
    id: v.id("_storage"),
  },
  handler: async (ctx, { id }) => {
    const img = await ctx.storage.get(id);
    if (!img) {
      throw new Error("could not fetch img");
    }
    /*
      const buf = await sharp(img)
          .greyscale()
          .toBuffer();
       */
  },
});
