"use node";

import { convexGateway } from "@convex-dev/ai-sdk-provider";
import { generateText } from "ai";
import { actionGeneric } from "convex/server";
import { v } from "convex/values";

export const chatCompletion = actionGeneric({
  args: {
    prompt: v.string(),
  },
  returns: v.string(),
  handler: async (_ctx, { prompt }) => {
    const { text } = await generateText({
      model: convexGateway("openai/gpt-4o-mini"),
      prompt,
    });
    if (!text) {
      throw new Error("The AI response did not contain text.");
    }
    return text;
  },
});
