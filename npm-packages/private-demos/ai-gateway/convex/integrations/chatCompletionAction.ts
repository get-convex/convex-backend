import { actionGeneric } from "convex/server";
import { v } from "convex/values";

type ChatCompletion = (prompt: string, gatewayUrl?: string) => Promise<string>;

export function defineChatCompletionAction(chatCompletion: ChatCompletion) {
  return actionGeneric({
    args: {
      prompt: v.string(),
      gatewayUrl: v.optional(v.string()),
    },
    returns: v.string(),
    handler: async (_ctx, { prompt, gatewayUrl }) => {
      return await chatCompletion(prompt, gatewayUrl);
    },
  });
}
