import { Base64, v } from "convex/values";
import { queryPrivateSystem } from "../secretSystemTables";
import { decodeId } from "id-encoding";
import { DataModel } from "../../_generated/dataModel";
import { GenericDatabaseReader } from "convex/server";

async function getTableId(
  db: GenericDatabaseReader<DataModel>,
  tableName: string,
  tableNamespace: string | null,
): Promise<string | undefined> {
  // Get the table id for the tablename
  const tablesWithName = await db
    .query("_tables")
    .withIndex("by_name", (q) => q.eq("name", tableName))
    .filter((q) => q.eq(q.field("state"), "active"))
    .collect();
  let tableId;
  if (tableNamespace === null) {
    const tables = tablesWithName.filter(
      (table) => table.namespace === undefined,
    );
    if (tables.length !== 1) {
      return undefined;
    }
    tableId = tables[0]._id;
  } else {
    const tables = tablesWithName.filter(
      (table) => table.namespace && table.namespace.id === tableNamespace,
    );
    if (tables.length !== 1) {
      return undefined;
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
    // Pass the `componentId` for this arg.
    // Note that this arg is named `tableNamespace` not `componentId` because if it is `componentId`,
    // the queries will be executed within the component's table namespace,
    // which doesn't have the `_index` or `_index_backfills` tables
    // We only need this argument to get the correct tableId.
    tableNamespace: v.union(v.string(), v.null()),
  },
  handler: async ({ db }, { tableName, tableNamespace }) => {
    if (!tableName) {
      return undefined;
    }
    const tableId = await getTableId(db, tableName, tableNamespace);
    if (!tableId) {
      return undefined;
    }
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
        function getIndexFieldsAndState(config: typeof index.config): {
          fields:
            | string[]
            | { searchField: string; filterFields: string[] }
            | {
                vectorField: string;
                filterFields: string[];
                dimensions: number;
              };
          state: "backfilling" | "backfilled" | "done";
          staged: boolean;
        } {
          switch (config.type) {
            case "database": {
              const stateType = config.onDiskState.type;
              let staged;
              let state;
              switch (stateType) {
                case "Backfilling":
                  staged = config.onDiskState.backfillState.staged ?? false;
                  state = "backfilling" as const;
                  break;
                case "Backfilled2":
                  staged = config.onDiskState.staged ?? false;
                  state = "backfilled" as const;
                  break;
                default:
                  staged = false;
                  state = "done" as const;
              }
              return { fields: config.fields, state, staged };
            }
            case "search": {
              const stateType = config.onDiskState.state;
              const state =
                stateType === "backfilling" || stateType === "backfilling2"
                  ? ("backfilling" as const)
                  : stateType === "backfilled" || stateType === "backfilled2"
                    ? ("backfilled" as const)
                    : ("done" as const);
              const fields = {
                searchField: config.searchField,
                filterFields: config.filterFields,
              };
              const staged =
                stateType === "backfilling" ||
                stateType === "backfilling2" ||
                stateType === "backfilled2"
                  ? (config.onDiskState.staged ?? false)
                  : false;
              return {
                fields,
                state,
                staged,
              };
            }
            case "vector": {
              const stateType = config.onDiskState.state;
              const state =
                stateType === "backfilling"
                  ? ("backfilling" as const)
                  : stateType === "backfilled" || stateType === "backfilled2"
                    ? ("backfilled" as const)
                    : ("done" as const);
              const staged =
                stateType === "backfilling" ||
                stateType === "backfilled" ||
                stateType === "backfilled2"
                  ? (config.onDiskState.staged ?? false)
                  : false;
              return {
                fields: {
                  vectorField: config.vectorField,
                  filterFields: config.filterFields,
                  dimensions: Number(config.dimensions),
                },
                state,
                staged,
              };
            }
            default: {
              const _typecheck: never = config;
              throw new Error(`Unknown index type`);
            }
          }
        }

        const { fields, state, staged } = getIndexFieldsAndState(index.config);
        if (state === "backfilling") {
          const indexBackfill = await db
            .query("_index_backfills")
            .withIndex("by_index_id", (q) => q.eq("indexId", index._id))
            .unique();
          const stats = indexBackfill
            ? {
                numDocsIndexed: Number(indexBackfill.numDocsIndexed),
                totalDocs: indexBackfill.totalDocs
                  ? Number(indexBackfill.totalDocs)
                  : null,
              }
            : undefined;
          return {
            name: index.descriptor,
            staged,
            fields,
            backfill: { state, stats: stats },
          };
        }
        return {
          name: index.descriptor,
          staged,
          fields,
          backfill: { state },
        };
      }),
    );
  },
});
