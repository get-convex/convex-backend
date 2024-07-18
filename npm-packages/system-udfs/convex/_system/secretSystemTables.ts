import {
  GenericDataModel,
  GenericDatabaseReader,
  SystemDataModel,
  DefaultFunctionArgs,
  TableNamesInDataModel,
  currentSystemUdfInComponent,
} from "convex/server";
import { GenericValidator } from "convex/values";
import { query as baseQuery, queryGeneric as baseQueryGeneric } from "./server";
import { Id } from "../_generated/dataModel";

// This set must be kept up-to-date to prevent accidental access to secret
// system tables in system UDFs.
const VIRTUAL_TABLES: Set<TableNamesInDataModel<SystemDataModel>> = new Set([
  "_storage",
  "_scheduled_functions",
]);

function isValidVirtualTable(table: string) {
  return (
    !table.startsWith("_") ||
    VIRTUAL_TABLES.has(table as TableNamesInDataModel<SystemDataModel>)
  );
}

const GUARANTEED_NONEXISTENT_TABLE = "_guaranteed_nonexistent_table_2508b1e2";

/**
 * System tables can only be queried with `db.system`, but `db.system` uses the public types of virtual tables,
 * the way we intend to expose system tables to developers.
 * In order to use system tables types (which we manually keep updated in dashboard/convex/schema.ts)
 * which are not exposed in `convex/server`, use normal data model types with the db.system at runtime.
 */

// db.system but filtered to private (non-virtual) system tables.
function maskPrivateSystem<T extends GenericDataModel>(
  db: GenericDatabaseReader<T>,
): GenericDatabaseReader<T> {
  return {
    query(tableName: string) {
      if (isValidVirtualTable(tableName)) {
        return db.system.query(GUARANTEED_NONEXISTENT_TABLE as any);
      }
      return db.system.query(tableName as any);
    },
    async get(id: Id<any>) {
      for (const validTable of VIRTUAL_TABLES) {
        if (db.system.normalizeId(validTable, id)) {
          return null;
        }
      }
      return db.system.get(id);
    },
    normalizeId(tableName: string, id: Id<any>) {
      if (isValidVirtualTable(tableName)) {
        return null;
      }
      return db.system.normalizeId(tableName as any, id);
    },
  } as typeof db;
}

// db.system but filtered to public (virtual) system tables.
function maskPublicSystem<T extends GenericDataModel>(
  db: GenericDatabaseReader<T>,
): GenericDatabaseReader<T>["system"] {
  return {
    query(tableName: string) {
      if (!isValidVirtualTable(tableName)) {
        return db.system.query(GUARANTEED_NONEXISTENT_TABLE as any);
      }
      return db.system.query(tableName as any);
    },
    async get(id: Id<TableNamesInDataModel<SystemDataModel>>) {
      for (const validTable of VIRTUAL_TABLES) {
        if (db.system.normalizeId(validTable, id)) {
          return db.system.get(id);
        }
      }
      return null;
    },
    normalizeId(
      tableName: TableNamesInDataModel<SystemDataModel>,
      id: Id<TableNamesInDataModel<SystemDataModel>>,
    ) {
      if (!isValidVirtualTable(tableName)) {
        return null;
      }
      return db.system.normalizeId(tableName, id);
    },
  } as GenericDatabaseReader<T>["system"];
}

type FunctionDefinition = {
  args: Record<string, GenericValidator>;
  handler: (ctx: any, args: DefaultFunctionArgs) => any;
};

const queryWithComponent = ((functionDefinition: FunctionDefinition) => {
  return baseQuery({
    args: functionDefinition.args,
    handler: async (ctx: any, args: any) => {
      if (
        "componentId" in args &&
        args.componentId !== null &&
        args.componentId !== undefined
      ) {
        const ref = currentSystemUdfInComponent(args.componentId);
        return await ctx.runQuery(ref, { ...args, componentId: null });
      }
      return functionDefinition.handler(ctx, args);
    },
  });
}) as typeof baseQuery;

/// `queryPrivateSystem` is for querying private system tables.
/// Access private system tables with `db.get/db.query`, not `db.system`,
/// although db.system is used under the hood.
/// In a `queryPrivateSystem` there is no access to user tables or public system
/// tables. For those, use `queryGeneric` instead.
export const queryPrivateSystem = ((functionDefinition: FunctionDefinition) => {
  if (!("args" in functionDefinition)) {
    throw new Error("args validator required for system udf");
  }
  return queryWithComponent({
    args: functionDefinition.args,
    handler: (ctx: any, args: any) => {
      return functionDefinition.handler(
        { ...ctx, db: maskPrivateSystem(ctx.db) },
        args,
      );
    },
  });
}) as typeof baseQuery;

const queryGenericWithComponent = ((functionDefinition: FunctionDefinition) => {
  return baseQueryGeneric({
    args: functionDefinition.args,
    handler: async (ctx: any, args: any) => {
      if (
        "componentId" in args &&
        args.componentId !== null &&
        args.componentId !== undefined
      ) {
        const ref = currentSystemUdfInComponent(args.componentId);
        return await ctx.runQuery(ref, { ...args, componentId: null });
      }
      return functionDefinition.handler(ctx, args);
    },
  });
}) as typeof baseQueryGeneric;

/// `queryGeneric` is a query that the developer could write themselves.
/// It does not access private system tables, so `db.get` and `db.system.get`
/// only operate on user tables and public system tables.
export const queryGeneric = ((functionDefinition: FunctionDefinition) => {
  if (!("args" in functionDefinition)) {
    throw new Error("args validator required for system udf");
  }
  return queryGenericWithComponent({
    args: functionDefinition.args,
    handler: (ctx: any, args: any) => {
      return functionDefinition.handler(
        {
          ...ctx,
          db: {
            ...ctx.db,
            system: maskPublicSystem(ctx.db),
            privateSystem: maskPrivateSystem(ctx.db),
          },
        },
        args,
      );
    },
  });
}) as typeof baseQueryGeneric;
