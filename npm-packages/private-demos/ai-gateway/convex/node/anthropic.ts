"use node";

import { listModels as listModelsWithAnthropic } from "../integrations/anthropic";
import { defineListModelsAction } from "../integrations/listModelsAction";

export const listModels = defineListModelsAction(listModelsWithAnthropic);
