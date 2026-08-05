import { actionGeneric } from "convex/server";
import { v } from "convex/values";
import type { ListModels } from "./modelList";

export function defineListModelsAction(listModels: ListModels) {
  return actionGeneric({
    args: {
      gatewayUrl: v.optional(v.string()),
    },
    returns: v.object({
      object: v.string(),
      count: v.number(),
      firstModel: v.union(v.string(), v.null()),
    }),
    handler: async (_ctx, { gatewayUrl }) => {
      return await listModels(gatewayUrl);
    },
  });
}
