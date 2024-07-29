import {
  convexToJson,
  GenericId,
  jsonToConvex,
  Value,
} from "../../values/index.js";
import { performAsyncSyscall, performSyscall } from "./syscall.js";
import {
  GenericDatabaseReader,
  GenericDatabaseReaderWithTable,
  GenericDatabaseWriter,
  GenericDatabaseWriterWithTable,
} from "../database.js";
import { QueryInitializerImpl } from "./query_impl.js";
import { GenericDataModel, GenericDocument } from "../data_model.js";
import { validateArg } from "./validate.js";
import { version } from "../../index.js";
import { patchValueToJson } from "../../values/value.js";

async function get(id: GenericId<string>, isSystem: boolean) {
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
}

export function setupReader(): GenericDatabaseReader<GenericDataModel> {
  const reader = (
    isSystem = false,
  ): GenericDatabaseReader<GenericDataModel> &
    GenericDatabaseReaderWithTable<GenericDataModel> => {
    return {
      get: async (id: GenericId<string>) => {
        return await get(id, isSystem);
      },
      query: (tableName: string) => {
        return new TableReader(tableName, isSystem).query();
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
      table: (tableName) => {
        return new TableReader(tableName, isSystem);
      },
    };
  };
  const { system: _, ...rest } = reader(true);
  const r = reader();
  r.system = rest as any;
  return r;
}

async function insert(tableName: string, value: any) {
  if (tableName.startsWith("_")) {
    throw new Error("System tables (prefixed with `_`) are read-only.");
  }
  validateArg(tableName, 1, "insert", "table");
  validateArg(value, 2, "insert", "value");
  const syscallJSON = await performAsyncSyscall("1.0/insert", {
    table: tableName,
    value: convexToJson(value),
  });
  const syscallResult = jsonToConvex(syscallJSON) as any;
  return syscallResult._id;
}

async function patch(id: any, value: any) {
  validateArg(id, 1, "patch", "id");
  validateArg(value, 2, "patch", "value");
  await performAsyncSyscall("1.0/shallowMerge", {
    id: convexToJson(id),
    value: patchValueToJson(value as Value),
  });
}

async function replace(id: any, value: any) {
  validateArg(id, 1, "replace", "id");
  validateArg(value, 2, "replace", "value");
  await performAsyncSyscall("1.0/replace", {
    id: convexToJson(id),
    value: convexToJson(value),
  });
}

async function delete_(id: any) {
  validateArg(id, 1, "delete", "id");
  await performAsyncSyscall("1.0/remove", { id: convexToJson(id) });
}

export function setupWriter(): GenericDatabaseWriter<GenericDataModel> &
  GenericDatabaseWriterWithTable<GenericDataModel> {
  const reader = setupReader();
  return {
    get: reader.get,
    query: reader.query,
    normalizeId: reader.normalizeId,
    system: reader.system as any,
    insert: async (table, value) => {
      return await insert(table, value);
    },
    patch: async (id, value) => {
      return await patch(id, value);
    },
    replace: async (id, value) => {
      return await replace(id, value);
    },
    delete: async (id) => {
      return await delete_(id);
    },
    table: (tableName) => {
      return new TableWriter(tableName, false);
    },
  };
}

class TableReader {
  constructor(
    protected readonly tableName: string,
    protected readonly isSystem: boolean,
  ) {}

  async get(id: GenericId<string>) {
    return get(id, this.isSystem);
  }

  query() {
    const accessingSystemTable = this.tableName.startsWith("_");
    if (accessingSystemTable !== this.isSystem) {
      throw new Error(
        `${
          accessingSystemTable ? "System" : "User"
        } tables can only be accessed from db.${
          this.isSystem ? "" : "system."
        }query().`,
      );
    }
    return new QueryInitializerImpl(this.tableName);
  }
}

class TableWriter extends TableReader {
  async insert(value: any) {
    return insert(this.tableName, value);
  }
  async patch(id: any, value: any) {
    return patch(id, value);
  }
  async replace(id: any, value: any) {
    return replace(id, value);
  }
  async delete(id: any) {
    return delete_(id);
  }
}
