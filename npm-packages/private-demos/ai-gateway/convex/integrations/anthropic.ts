"use node";

import Anthropic from "@anthropic-ai/sdk";
import { getServiceToken } from "convex/server";
import {
  LOCAL_GATEWAY_URL,
  type ModelListSummary,
  summarizeModels,
} from "./modelList";

export async function listModels(
  gatewayUrl = LOCAL_GATEWAY_URL,
): Promise<ModelListSummary> {
  const anthropic = new Anthropic({
    baseURL: gatewayUrl,
    apiKey: null,
    authToken: await getServiceToken("ai"),
  });
  const models = await anthropic.models.list();
  return summarizeModels({
    object: "list",
    data: models.data,
  });
}
