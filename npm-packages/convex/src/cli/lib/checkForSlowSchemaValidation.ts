import { chalkStderr } from "chalk";
import { Context } from "../../bundler/context.js";
import { logMessage, logVerbose } from "../../bundler/log.js";
import { Span } from "./tracing.js";
import { StartPushRequest } from "./deployApi/startPush.js";
import {
  EvaluateSchemaResponse,
  TablePrediction,
} from "./deployApi/evaluateSchema.js";
import { evaluateSchema } from "./deploy2.js";
import { formatSize } from "./utils/utils.js";

const MIN_BYTES_FOR_SCHEMA_WALK_WARNING = 1 << 27; // 128 MiB

export async function checkForSlowSchemaValidation({
  ctx,
  span,
  request,
  options,
}: {
  ctx: Context;
  span: Span;
  request: StartPushRequest;
  options: {
    url: string;
    deploymentName: string | null;
    adminKey: string;
  };
}): Promise<void> {
  const response = await evaluateSchemaBestEffort({
    ctx,
    span,
    request,
    options,
  });
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

// This warning is advisory: if evaluation fails, fall back to skipping it
// rather than aborting the push over a check that isn't required for
// correctness. Plausibly transient failures — network errors and 5xx
// responses (e.g. table summaries still bootstrapping) — are retried with
// backoff by the shared fetch wrapper (see `pushCode`); anything else is
// deterministic, including a 2xx whose shape the CLI doesn't recognize, so
// it skips the check immediately.
async function evaluateSchemaBestEffort({
  ctx,
  span,
  request,
  options,
}: {
  ctx: Context;
  span: Span;
  request: StartPushRequest;
  options: {
    url: string;
    deploymentName: string | null;
    adminKey: string;
  };
}): Promise<EvaluateSchemaResponse | null> {
  try {
    return await evaluateSchema(
      ctx,
      span,
      request,
      options,
      /* bestEffort */ true,
    );
  } catch (error: unknown) {
    logVerbose(
      `Skipping slow schema validation check: ${error instanceof Error ? error.message : String(error)}`,
    );
    return null;
  }
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
  const envValue = process.env.CONVEX_MIN_BYTES_FOR_SCHEMA_WALK_WARNING;
  if (envValue !== undefined) {
    const parsed = parseInt(envValue, 10);
    if (!isNaN(parsed)) {
      return parsed;
    }
  }
  return MIN_BYTES_FOR_SCHEMA_WALK_WARNING;
}
