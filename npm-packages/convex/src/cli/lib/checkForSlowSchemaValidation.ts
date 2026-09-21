import { chalkStderr } from "chalk";
import { logMessage } from "../../bundler/log.js";
import { Span } from "./tracing.js";
import { TablePrediction } from "./deployApi/evaluateSchema.js";
import { SchemaEvaluation } from "./schemaEvaluation.js";
import { formatSize, integerFromEnv } from "./utils/utils.js";

const MIN_BYTES_FOR_SCHEMA_WALK_WARNING = 1 << 27; // 128 MiB

export async function checkForSlowSchemaValidation({
  span,
  schemaEvaluation,
}: {
  span: Span;
  schemaEvaluation: SchemaEvaluation;
}): Promise<void> {
  const response = await schemaEvaluation(span);
  if (response === null) {
    return;
  }

  const walkedTables = Object.entries(
    response.componentSchemaEvaluations,
  ).flatMap(([componentPath, prediction]) =>
    prediction.tables
      .filter((table) => table.outcome === "mustWalk")
      .map((table) => ({ componentPath, table })),
  );
  if (walkedTables.length === 0) {
    return;
  }

  const totalBytes = walkedTables.reduce(
    (sum, { table }) => sum + table.sizeBytes,
    0,
  );
  if (totalBytes < minBytesForSchemaWalkWarning()) {
    return;
  }

  logMessage(`⚠️  This schema change requires checking every document in the following ${walkedTables.length === 1 ? "table" : "tables"} against your new schema, totaling ${chalkStderr.yellowBright(formatSize(totalBytes))}. This deploy may take a while:

${walkedTables
  .map(({ componentPath, table }) =>
    formatWalkedTable({ componentPath, table }),
  )
  .join("\n")}
`);
}

function formatWalkedTable({
  componentPath,
  table,
}: {
  componentPath: string;
  table: TablePrediction;
}) {
  const componentPrefix =
    componentPath !== "" ? `${chalkStderr.gray(componentPath)}: ` : "";
  const docsFormatted = `${table.numDocs.toLocaleString()} documents, `;
  const sizeFormatted = formatSize(table.sizeBytes);
  return `  ${componentPrefix}${table.name} (${docsFormatted}${sizeFormatted})`;
}

function minBytesForSchemaWalkWarning(): number {
  return integerFromEnv(
    "CONVEX_MIN_BYTES_FOR_SCHEMA_WALK_WARNING",
    MIN_BYTES_FOR_SCHEMA_WALK_WARNING,
  );
}
