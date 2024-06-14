import { z } from "zod";
import { componentDefinitionPath } from "./paths.js";
import { moduleConfig } from "./modules.js";

export const appDefinitionConfig = z.object({
  definition: z.nullable(moduleConfig),
  dependencies: z.array(componentDefinitionPath),
  auth: z.nullable(moduleConfig),
  schema: z.nullable(moduleConfig),
  functions: z.array(moduleConfig),
});
export type AppDefinitionConfig = z.infer<typeof appDefinitionConfig>;

export const componentDefinitionConfig = z.object({
  definitionPath: componentDefinitionPath,
  definition: moduleConfig,
  dependencies: z.array(componentDefinitionPath),
  schema: z.nullable(moduleConfig),
  functions: z.array(moduleConfig),
});
export type ComponentDefinitionConfig = z.infer<
  typeof componentDefinitionConfig
>;
