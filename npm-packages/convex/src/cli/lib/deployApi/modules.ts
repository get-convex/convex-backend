import { z } from "zod";

export const moduleEnvironment = z.union([
  z.literal("isolate"),
  z.literal("node"),
]);
export type ModuleEnvironment = z.infer<typeof moduleEnvironment>;

export const moduleConfig = z.object({
  path: z.string(),
  source: z.string(),
  sourceMap: z.optional(z.string()),
  environment: moduleEnvironment,
});
export type ModuleConfig = z.infer<typeof moduleConfig>;

export const nodeDependency = z.object({
  name: z.string(),
  version: z.string(),
});
export type NodeDependency = z.infer<typeof nodeDependency>;

export const udfConfig = z.object({
  serverVersion: z.string(),
  // RNG seed encoded as Convex bytes in JSON.
  importPhaseRngSeed: z.any(),
  // Timestamp encoded as a Convex Int64 in JSON.
  importPhaseUnixTimestamp: z.any(),
});
export type UdfConfig = z.infer<typeof udfConfig>;

export const sourcePackage = z.any();
export type SourcePackage = z.infer<typeof sourcePackage>;

export const visibility = z.union([
  z.object({ kind: z.literal("public") }),
  z.object({ kind: z.literal("internal") }),
]);
export type Visibility = z.infer<typeof visibility>;

export const analyzedFunction = z.object({
  name: z.string(),
  pos: z.any(),
  udfType: z.union([
    z.literal("Query"),
    z.literal("Mutation"),
    z.literal("Action"),
  ]),
  visibility: z.nullable(visibility),
  args: z.nullable(z.string()),
  returns: z.nullable(z.string()),
});
export type AnalyzedFunction = z.infer<typeof analyzedFunction>;

export const analyzedModule = z.object({
  functions: z.array(analyzedFunction),
  httpRoutes: z.any(),
  cronSpecs: z.any(),
  sourceMapped: z.any(),
});
export type AnalyzedModule = z.infer<typeof analyzedModule>;
