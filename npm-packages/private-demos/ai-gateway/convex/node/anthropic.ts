"use node";

import { defineChatCompletionAction } from "../integrations/chatCompletionAction";
import { defineListModelsAction } from "../integrations/listModelsAction";
import {
  chatCompletion as chatCompletionWithAnthropic,
  listModels as listModelsWithAnthropic,
} from "../integrations/anthropic";

export const listModels = defineListModelsAction(listModelsWithAnthropic);
export const chatCompletion = defineChatCompletionAction(
  chatCompletionWithAnthropic,
);
