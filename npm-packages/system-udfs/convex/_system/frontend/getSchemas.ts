import { DatabaseReader } from "../../_generated/server";
import { Doc } from "../../_generated/dataModel";
import { queryPrivateSystem } from "../secretSystemTables";
import { v } from "convex/values";

type SchemaMetadata = Doc<"_schemas">;

export const getSchemaByState = async (
  db: DatabaseReader,
  state: SchemaMetadata["state"]["state"],
) =>
  await db
    .query("_schemas")
    .withIndex("by_state", (q) => q.eq("state", { state }))
    .unique();

export default queryPrivateSystem("ViewData")({
  args: { componentId: v.optional(v.union(v.string(), v.null())) },
  handler: async function ({ db }): Promise<{
    active?: string;
    inProgress?: string;
  }> {
    const active = await getSchemaByState(db, "active");
    const pending = await getSchemaByState(db, "pending");
    const validated = await getSchemaByState(db, "validated");

    if (pending && validated) {
      throw new Error("Unexpectedly found both pending and validated schemas");
    }

    return {
      active: active?.schema,
      inProgress: pending?.schema || validated?.schema,
    };
  },
});

type ValidationRow = Awaited<ReturnType<typeof validationProgress>>;

/// Per-table validation attempts for the pending schema, or null when no
/// schema is pending or the worker hasn't started walking tables yet.
async function pendingValidationRows(
  db: DatabaseReader,
): Promise<ValidationRow[] | null> {
  const pending = await getSchemaByState(db, "pending");
  if (!pending) {
    return null;
  }
  const attempts = await db
    .query("_schema_validations")
    .withIndex("by_schema_id_and_table_name", (q) =>
      q.eq("schemaId", pending._id),
    )
    .collect();
  const rows = await Promise.all(
    attempts.map((attempt) => validationProgress(db, attempt)),
  );
  return rows.length === 0 ? null : rows;
}

function aggregateProgress(rows: ValidationRow[]) {
  return {
    numDocsValidated: rows.reduce(
      (sum, row) => sum + Number(row.numDocsValidated),
      0,
    ),
    totalDocs: rows.every((row) => row.totalDocs !== null)
      ? rows.reduce((sum, row) => sum + Number(row.totalDocs), 0) || null
      : null,
  };
}

export const schemaValidationProgress = queryPrivateSystem("ViewData")({
  args: { componentId: v.optional(v.union(v.string(), v.null())) },
  handler: async function ({
    db,
  }): Promise<{ numDocsValidated: number; totalDocs: number | null } | null> {
    const rows = await pendingValidationRows(db);
    return rows === null ? null : aggregateProgress(rows);
  },
});

export type TableValidationProgress = {
  tableName: string;
  state: "pending" | "valid" | "failed";
  error?: string;
  numDocsValidated: number;
  totalDocs: number | null;
};

/// The aggregate progress plus one entry per table the pending schema is
/// validating, sorted by table name.
export const schemaValidationProgressByTable = queryPrivateSystem("ViewData")({
  args: { componentId: v.optional(v.union(v.string(), v.null())) },
  handler: async function ({ db }): Promise<{
    numDocsValidated: number;
    totalDocs: number | null;
    tables: TableValidationProgress[];
  } | null> {
    const rows = await pendingValidationRows(db);
    if (rows === null) {
      return null;
    }
    const tables = rows
      .map((row) => ({
        tableName: row.tableName,
        state: row.state.state,
        ...(row.state.state === "failed" ? { error: row.state.error } : {}),
        numDocsValidated: Number(row.numDocsValidated),
        totalDocs: row.totalDocs === null ? null : Number(row.totalDocs),
      }))
      .sort((a, b) => a.tableName.localeCompare(b.tableName));
    return { ...aggregateProgress(rows), tables };
  },
});

async function validationProgress(
  db: DatabaseReader,
  attempt: Doc<"_schema_validations">,
) {
  const progress = await db
    .query("_schema_validation_progress")
    .withIndex("by_validation_id", (q) => q.eq("validationId", attempt._id))
    .unique();
  return {
    ...attempt,
    numDocsValidated: progress?.numDocsValidated ?? BigInt(0),
    totalDocs: progress?.totalDocs ?? null,
  };
}
