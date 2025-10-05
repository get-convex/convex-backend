"use node";
// waiting for a better quickfix to enforce this
/* eslint-disable @convex-dev/no-old-registered-function-syntax */
import { action } from "./_generated/server";
// import sharp from 'sharp';

export const getUploadUrl = action(async (ctx) => {
  return await ctx.storage.generateUploadUrl();
});

export const l = action(async (ctx, { id }: { id: string }) => {
  const img = await ctx.storage.get(id);
  if (!img) {
    throw new Error("could not fetch img");
  }
  /*
    const buf = await sharp(img)
        .greyscale()
        .toBuffer();
     */
});
