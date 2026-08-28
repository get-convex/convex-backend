"use node";

import Anthropic from "@anthropic-ai/sdk";
import { getServiceToken } from "convex/server";
import {
  LOCAL_GATEWAY_URL,
  type ModelListSummary,
  summarizeModels,
} from "./modelList";

async function createAnthropicClient(gatewayUrl: string): Promise<Anthropic> {
  return new Anthropic({
    baseURL: gatewayUrl,
    apiKey: null,
    authToken: await getServiceToken("ai-gateway"),
  });
}

export async function listModels(
  gatewayUrl = LOCAL_GATEWAY_URL,
): Promise<ModelListSummary> {
  const anthropic = await createAnthropicClient(gatewayUrl);
  const models = await anthropic.models.list();
  return summarizeModels({
    object: "list",
    data: models.data,
  });
}

export async function chatCompletion(
  prompt: string,
  gatewayUrl = LOCAL_GATEWAY_URL,
): Promise<string> {
  const anthropic = await createAnthropicClient(gatewayUrl);
  const message = await anthropic.messages.create({
    model: "anthropic/claude-haiku-4.5",
    max_tokens: 128,
    messages: [{ role: "user", content: prompt }],
  });
  const text = message.content
    .filter((block) => block.type === "text")
    .map((block) => block.text)
    .join("");
  if (!text) {
    throw new Error("The AI response did not contain text.");
  }
  return text;
}
