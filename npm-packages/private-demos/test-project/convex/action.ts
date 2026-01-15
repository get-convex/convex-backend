import { action } from "./_generated/server";
import { v } from "convex/values";

const ENDPOINT = "https://commons.wikimedia.org/w/api.php";

export const fetchPictureOfTheDay = action({
  args: {
    date: v.optional(v.string()), // ISO date string, defaults to today
  },
  handler: async (_ctx, args) => {
    const dateStr = args.date || new Date().toISOString().split("T")[0];
    const title = `Template:Potd/${dateStr}`;

    // First query: Get the image filename from the template
    const filenameParams = new URLSearchParams({
      action: "query",
      format: "json",
      formatversion: "2",
      prop: "images",
      titles: title,
    });

    const filenameResponse = await fetch(`${ENDPOINT}?${filenameParams}`);
    const filenameData = await filenameResponse.json();

    const pages = filenameData.query?.pages;
    if (!pages || pages.length === 0 || !pages[0].images) {
      throw new Error(`No picture found for date: ${dateStr}`);
    }

    const filename = pages[0].images[0].title;
    const imagePageUrl = `https://commons.wikimedia.org/wiki/${encodeURIComponent(title)}`;

    // Second query: Get the image URL from the filename
    const imageUrlParams = new URLSearchParams({
      action: "query",
      format: "json",
      prop: "imageinfo",
      iiprop: "url",
      titles: filename,
    });

    const imageUrlResponse = await fetch(`${ENDPOINT}?${imageUrlParams}`);
    const imageUrlData = await imageUrlResponse.json();

    const pageValues = Object.values(imageUrlData.query.pages);
    const imagePage = pageValues[0] as any;
    const imageUrl = imagePage.imageinfo[0].url;

    return {
      filename,
      imagePageUrl,
      imageSrc: imageUrl,
      date: dateStr,
    };
  },
});

export const sendWebhookMessage = action({
  args: {
    webhookUrl: v.string(),
    message: v.string(),
    imageUrl: v.optional(v.string()),
  },
  handler: async (_ctx, args) => {
    const payload: any = {
      content: args.message,
    };

    // Add embed with image if imageUrl is provided
    if (args.imageUrl) {
      payload.embeds = [
        {
          image: {
            url: args.imageUrl,
          },
        },
      ];
    }

    const response = await fetch(args.webhookUrl, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
      throw new Error(`Failed to send webhook: ${response.statusText}`);
    }

    return { success: true };
  },
});
