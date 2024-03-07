import {
  convexToJson,
  GenericId,
  jsonToConvex,
  Value,
} from "../../values/index.js";
import { performAsyncSyscall, performSyscall } from "./syscall.js";
import { GenericDatabaseReader, GenericDatabaseWriter } from "../database.js";
import { QueryInitializerImpl } from "./query_impl.js";
import { GenericDataModel, GenericDocument } from "../data_model.js";
import { validateArg } from "./validate.js";
import { version } from "../../index.js";
import { patchValueToJson } from "../../values/value.js";

export function setupReader(): GenericDatabaseReader<GenericDataModel> {
  const reader = (
    isSystem = false,
  ): GenericDatabaseReader<GenericDataModel> => {
    return {
      get: async (id: GenericId<string>) => {
        validateArg(id, 1, "get", "id");
        if (typeof id !== "string") {
          throw new Error(
            `Invalid argument \`id\` for \`db.get\`, expected string but got '${typeof id}': ${
              id as any
            }`,
          );
        }
        const args = {
          id: convexToJson(id),
          isSystem,
          version,
        };
        const syscallJSON = await performAsyncSyscall("1.0/get", args);

        return jsonToConvex(syscallJSON) as GenericDocument;
      },
      query: (tableName: string) => {
        const accessingSystemTable = tableName.startsWith("_");
        if (accessingSystemTable !== isSystem) {
          throw new Error(
            `${
              accessingSystemTable ? "System" : "User"
            } tables can only be accessed from db.${
              isSystem ? "" : "system."
            }query().`,
          );
        }
        return new QueryInitializerImpl(tableName);
      },
      normalizeId: <TableName extends string>(
        tableName: TableName,
        id: string,
      ): GenericId<TableName> | null => {
        validateArg(tableName, 1, "normalizeId", "tableName");
        validateArg(id, 2, "normalizeId", "id");
        const accessingSystemTable = tableName.startsWith("_");
        if (accessingSystemTable !== isSystem) {
          throw new Error(
            `${
              accessingSystemTable ? "System" : "User"
            } tables can only be accessed from db.${
              isSystem ? "" : "system."
            }normalizeId().`,
          );
        }
        const syscallJSON = performSyscall("1.0/db/normalizeId", {
          table: tableName,
          idString: id,
        });
        const syscallResult = jsonToConvex(syscallJSON) as any;
        return syscallResult.id;
      },
      // We set the system reader on the next line
      system: null as any,
    };
  };
  const { system: _, ...rest } = reader(true);
  const r = reader();
  r.system = rest;
  return r;
}

export function setupWriter(): GenericDatabaseWriter<GenericDataModel> {
  const reader = setupReader();
  return {
    get: reader.get,
    query: reader.query,
    normalizeId: reader.normalizeId,
    system: reader.system,
    insert: async (table, value) => {
      if (table.startsWith("_")) {
        throw new Error("System tables (prefixed with `_`) are read-only.");
      }
      validateArg(table, 1, "insert", "table");
      validateArg(value, 2, "insert", "value");
      const syscallJSON = await performAsyncSyscall("1.0/insert", {
        table,
        value: convexToJson(value),
      });
      const syscallResult = jsonToConvex(syscallJSON) as any;
      return syscallResult._id;
    },
    patch: async (id, value) => {
      validateArg(id, 1, "patch", "id");
      validateArg(value, 2, "patch", "value");
      await performAsyncSyscall("1.0/shallowMerge", {
        id: convexToJson(id),
        value: patchValueToJson(value as Value),
      });
    },
    replace: async (id, value) => {
      validateArg(id, 1, "replace", "id");
      validateArg(value, 2, "replace", "value");
      await performAsyncSyscall("1.0/replace", {
        id: convexToJson(id),
        value: convexToJson(value),
      });
    },
    delete: async (id) => {
      validateArg(id, 1, "delete", "id");
      await performAsyncSyscall("1.0/remove", { id: convexToJson(id) });
    },
  };
}
