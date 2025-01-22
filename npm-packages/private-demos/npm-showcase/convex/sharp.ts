"use node";
import { action } from "./_generated/server";
// eslint-disable-next-line @typescript-eslint/no-var-requires
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
