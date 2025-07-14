import { Base64, v } from "convex/values";
import { queryPrivateSystem } from "../secretSystemTables";
import { decodeId } from "id-encoding";
import { DataModel } from "../../_generated/dataModel";
import { GenericDatabaseReader } from "convex/server";

async function getTableId(
  db: GenericDatabaseReader<DataModel>,
  tableName: string,
  componentId: string | null,
): Promise<string> {
  // Get the table id for the tablename
  const tablesWithName = await db
    .query("_tables")
    .withIndex("by_name", (q) => q.eq("name", tableName))
    .filter((q) => q.eq(q.field("state"), "active"))
    .collect();
  let tableId;
  if (componentId === null) {
    const tables = tablesWithName.filter(
      (table) => table.namespace === undefined,
    );
    if (tables.length !== 1) {
      throw new Error(
        "Table not found for tableName" + tableName + " in the root namespace ",
      );
    }
    tableId = tables[0]._id;
  } else {
    const tables = tablesWithName.filter(
      (table) => table.namespace && table.namespace.id === componentId,
    );
    if (tables.length !== 1) {
      throw new Error(
        "Table not found for tableName" +
          tableName +
          " in the componentId " +
          componentId,
      );
    }
    tableId = tables[0]._id;
  }
  const decodedId = decodeId(tableId);
  const tableInternalId = decodedId.internalId;
  const urlSafeInternalId =
    Base64.fromByteArrayUrlSafeNoPadding(tableInternalId);
  return urlSafeInternalId;
}

export default queryPrivateSystem({
  args: {
    tableName: v.optional(v.union(v.string(), v.null())),
    componentId: v.union(v.string(), v.null()),
  },
  handler: async ({ db }, { tableName, componentId }) => {
    if (!tableName) {
      return undefined;
    }
    const tableId = await getTableId(db, tableName, componentId);
    const indexes = await db
      .query("_index")
      .withIndex("by_id", (q) => q)
      .filter((q) => q.eq(q.field("table_id"), tableId))
      .collect();
    const userIndexes = indexes.filter(
      (index) =>
        index.descriptor !== "by_id" && index.descriptor !== "by_creation_time",
    );
    return Promise.all(
      userIndexes.map(async (index) => {
        function getIndexFields(config: typeof index.config) {
          switch (config.type) {
            case "database":
              return config.fields;
            case "search":
              return {
                searchField: config.searchField,
                filterFields: config.filterFields,
              };
            case "vector":
              return {
                vectorField: config.vectorField,
                filterFields: config.filterFields,
                dimensions: Number(config.dimensions),
              };
            default: {
              const _typecheck: never = config;
              throw new Error(`Unknown index type`);
            }
          }
        }

        const fields = getIndexFields(index.config);
        const state =
          index.config.onDiskState.type === "Backfilling"
            ? ("in_progress" as const)
            : ("done" as const);
        if (state === "in_progress") {
          const indexBackfill = await db
            .query("_index_backfills")
            .withIndex("by_index_id", (q) => q.eq("indexId", index._id))
            .unique();
          const stats = indexBackfill
            ? {
                numDocsIndexed: Number(indexBackfill.numDocsIndexed),
                totalDocs: Number(indexBackfill.totalDocs),
              }
            : undefined;
          return {
            name: index.descriptor,
            fields,
            backfill: { state, stats: stats },
          };
        }
        return {
          name: index.descriptor,
          fields,
          backfill: { state },
        };
      }),
    );
  },
});
