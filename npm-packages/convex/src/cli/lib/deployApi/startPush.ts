import { z } from "zod";
import { componentDefinitionPath } from "./paths.js";
import { nodeDependency, sourcePackage } from "./modules.js";
import { checkedComponent } from "./checkedComponent.js";
import { evaluatedComponentDefinition } from "./componentDefinition.js";
import {
  appDefinitionConfig,
  componentDefinitionConfig,
} from "./definitionConfig.js";
import { authInfo } from "./types.js";

export const startPushRequest = z.object({
  adminKey: z.string(),
  dryRun: z.boolean(),

  functions: z.string(),

  appDefinition: appDefinitionConfig,
  componentDefinitions: z.array(componentDefinitionConfig),

  nodeDependencies: z.array(nodeDependency),
});
export type StartPushRequest = z.infer<typeof startPushRequest>;

export const schemaChange = z.object({
  allocatedComponentIds: z.any(),
  schemaIds: z.any(),
});
export type SchemaChange = z.infer<typeof schemaChange>;

export const startPushResponse = z.object({
  externalDepsId: z.nullable(z.string()),
  componentDefinitionPackages: z.record(componentDefinitionPath, sourcePackage),

  appAuth: z.array(authInfo),
  analysis: z.record(componentDefinitionPath, evaluatedComponentDefinition),

  app: checkedComponent,

  schemaChange,
});
export type StartPushResponse = z.infer<typeof startPushResponse>;
