import { z } from "zod";
import { componentDefinitionPath, componentPath } from "./paths.js";
import { developerIndexConfig } from "./finishPush.js";
import { looseObject } from "./utils.js";

// Hand-mirrors `TableValidationOutcome` in
// `crates/common/src/schemas/mod.rs` (`#[serde(rename_all = "camelCase")]`),
// like the rest of this deployApi directory: there's no codegen sharing
// types between the Rust backend and this package.
export const tableValidationOutcome = z.enum([
  "notValidated",
  "supersetOfEnforced",
  "supersetOfShape",
  "mustWalk",
]);
export type TableValidationOutcome = z.infer<typeof tableValidationOutcome>;

export const indexChangePrediction = z.enum([
  "added",
  "identical",
  "enabled",
  "disabled",
  "dropped",
]);
export type IndexChangePrediction = z.infer<typeof indexChangePrediction>;

export const tablePrediction = looseObject({
  name: z.string(),
  outcome: tableValidationOutcome,
  numDocs: z.number(),
  sizeBytes: z.number(),
});
export type TablePrediction = z.infer<typeof tablePrediction>;

export const indexPrediction = z.intersection(
  developerIndexConfig,
  looseObject({
    change: indexChangePrediction,
    needsBackfill: z.boolean(),
    numDocs: z.number(),
  }),
);
export type IndexPrediction = z.infer<typeof indexPrediction>;

export const componentSchemaPrediction = looseObject({
  definitionPath: z.string(),
  schemaValidation: z.boolean(),
  tables: z.array(tablePrediction),
  indexes: z.array(indexPrediction),
});
export type ComponentSchemaPrediction = z.infer<
  typeof componentSchemaPrediction
>;

export const evaluateSchemaResponse = looseObject({
  componentSchemaEvaluations: z.record(
    componentPath,
    componentSchemaPrediction,
  ),
  newComponentDefinitions: z.array(componentDefinitionPath),
});
export type EvaluateSchemaResponse = z.infer<typeof evaluateSchemaResponse>;
