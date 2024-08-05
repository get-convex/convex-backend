import { z } from "zod";
import { canonicalizedModulePath, componentDefinitionPath } from "./paths.js";
import { Identifier, Reference, identifier, reference } from "./types.js";
import { analyzedModule, udfConfig } from "./modules.js";

export const componentArgumentValidator = z.object({
  type: z.literal("value"),
  // Validator serialized to JSON.
  value: z.string(),
});

export const componentDefinitionType = z.union([
  z.object({ type: z.literal("app") }),
  z.object({
    type: z.literal("childComponent"),
    name: identifier,
    args: z.array(z.tuple([identifier, componentArgumentValidator])),
  }),
]);

export const componentArgument = z.object({
  type: z.literal("value"),
  // Value serialized to JSON.
  value: z.string(),
});

export const componentInstantiation = z.object({
  name: identifier,
  path: componentDefinitionPath,
  args: z.nullable(z.array(z.tuple([identifier, componentArgument]))),
});

export type ComponentExports =
  | { type: "leaf"; leaf: Reference }
  | { type: "branch"; branch: [Identifier, ComponentExports][] };

export const componentExports: z.ZodType<ComponentExports> = z.lazy(() =>
  z.union([
    z.object({
      type: z.literal("leaf"),
      leaf: reference,
    }),
    z.object({
      type: z.literal("branch"),
      branch: z.array(z.tuple([identifier, componentExports])),
    }),
  ]),
);

export const componentDefinitionMetadata = z.object({
  path: componentDefinitionPath,
  definitionType: componentDefinitionType,
  childComponents: z.array(componentInstantiation),
  httpMounts: z.record(z.string(), reference),
  exports: z.object({
    type: z.literal("branch"),
    branch: z.array(z.tuple([identifier, componentExports])),
  }),
});

export const evaluatedComponentDefinition = z.object({
  definition: componentDefinitionMetadata,
  schema: z.any(),
  functions: z.record(canonicalizedModulePath, analyzedModule),
  udfConfig,
});
export type EvaluatedComponentDefinition = z.infer<
  typeof evaluatedComponentDefinition
>;
