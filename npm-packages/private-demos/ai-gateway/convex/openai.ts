import { defineListModelsAction } from "./integrations/listModelsAction";
import { listModels as listModelsWithOpenAi } from "./integrations/openai";

export const listModels = defineListModelsAction(listModelsWithOpenAi);
