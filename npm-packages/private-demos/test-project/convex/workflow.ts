import { WorkflowManager } from "@convex-dev/workflow";
import { api, components, internal } from "./_generated/api";
import { v } from "convex/values";
import { mutation } from "./_generated/server";

export const workflow = new WorkflowManager(components.workflow);

export const sendPictureOfTheDay = workflow.define({
  args: {
    date: v.optional(v.string()), // ISO date string, defaults to today
  },
  handler: async (step, args) => {
    // Step 1: Fetch the picture of the day
    const pictureData = await step.runAction(api.action.fetchPictureOfTheDay, {
      date: args.date,
    });

    // Step 2: Create a thumbnail from the fetched image
    const thumbnailData = await step.runAction(api.nodeAction.createThumbnail, {
      imageUrl: pictureData.imageSrc,
    });

    // Step 3: Send message to chat with thumbnail data URL
    const thumbnailDataUrl = `data:${thumbnailData.mimeType};base64,${thumbnailData.thumbnail}`;
    await step.runMutation(api.chat.sendMessage, {
      username: "Picture Bot",
      message: `Picture of the day for ${pictureData.date}: ${thumbnailDataUrl}`,
    });
  },
});

export const start = mutation({
  args: {},
  handler: async (ctx) => {
    await workflow.start(ctx, internal.workflow.sendPictureOfTheDay, {});
  },
});
