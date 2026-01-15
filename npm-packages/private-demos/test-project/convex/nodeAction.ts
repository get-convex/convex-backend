"use node";

import { v } from "convex/values";
import { action } from "./_generated/server";
import sharp from "sharp";

export const createThumbnail = action({
  args: {
    imageUrl: v.string(),
  },
  handler: async (_ctx, args) => {
    const response = await fetch(args.imageUrl);

    if (!response.ok) {
      throw new Error(`Failed to fetch image: ${response.statusText}`);
    }

    const imageBuffer = Buffer.from(await response.arrayBuffer());

    const thumbnailBuffer = await sharp(imageBuffer)
      .resize(200, 200, {
        fit: "cover",
        position: "center",
      })
      .jpeg({ quality: 80 })
      .toBuffer();

    return {
      thumbnail: thumbnailBuffer.toString("base64"),
      mimeType: "image/jpeg",
      originalUrl: args.imageUrl,
    };
  },
});
