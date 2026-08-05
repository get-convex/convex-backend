import { getServiceToken } from "convex/server";
import OpenAI from "openai";
import {
  LOCAL_GATEWAY_URL,
  type ModelListSummary,
  summarizeModels,
} from "./modelList";

export async function listModels(
  gatewayUrl = LOCAL_GATEWAY_URL,
): Promise<ModelListSummary> {
  const openai = new OpenAI({
    baseURL: `${gatewayUrl}/v1`,
    apiKey: () => getServiceToken("ai"),
  });
  return summarizeModels(await openai.models.list());
}
