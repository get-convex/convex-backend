"use node";

import { convexGateway } from "@convex-dev/ai-sdk-provider";
import { generateText } from "ai";
import { actionGeneric } from "convex/server";
import { v } from "convex/values";
import { internalAction } from "../_generated/server";

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

export const triageTicket = internalAction({
  args: { ticket: v.string() },
  returns: v.object({ priority: v.string() }),
  handler: async (_ctx, { ticket }) => {
    const { answers } = await convexGateway.decisions({
      model: "typesafe/jev-1.13",
      state: { ticket },
      questions: {
        priority: {
          type: "choice",
          instructions: "Choose the support ticket's priority.",
          criteria: {
            urgent: "An outage or data loss is blocking users.",
            normal: "A bug affects users but has a workaround.",
            low: "A question or feature request without immediate impact.",
          },
        },
      },
    });
    return { priority: answers.priority.choice };
  },
});
