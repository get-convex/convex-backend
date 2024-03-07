import { defineTable } from "convex/server";
import { v } from "convex/values";

const createEnvironmentVariable = v.object({
  action: v.literal("create_environment_variable"),
  member_id: v.int64(),
  metadata: v.object({
    variable_name: v.string(),
  }),
});

const deleteEnvironmentVariable = v.object({
  action: v.literal("delete_environment_variable"),
  member_id: v.int64(),
  metadata: v.object({
    variable_name: v.string(),
  }),
});

const updateEnvironmentVariable = v.object({
  action: v.literal("update_environment_variable"),
  member_id: v.int64(),
  metadata: v.object({
    variable_name: v.string(),
  }),
});

const replaceEnvironmentVariable = v.object({
  action: v.literal("replace_environment_variable"),
  member_id: v.int64(),
  metadata: v.object({
    previous_variable_name: v.string(),
    variable_name: v.string(),
  }),
});

const databaseIndex = v.object({
  name: v.optional(v.string()),
  type: v.literal("database"),
  fields: v.array(v.string()),
});

const searchIndex = v.object({
  name: v.optional(v.string()),
  type: v.literal("search"),
  searchField: v.string(),
  filterFields: v.array(v.string()),
});

const vectorIndex = v.object({
  name: v.optional(v.string()),
  type: v.literal("vector"),
  vectorField: v.string(),
  filterFields: v.array(v.string()),
  dimensions: v.number(),
});

export const indexMetadata = v.union(databaseIndex, searchIndex, vectorIndex);

export const buildIndexes = v.object({
  action: v.literal("build_indexes"),
  member_id: v.int64(),
  metadata: v.object({
    added_indexes: v.array(v.union(databaseIndex, searchIndex, vectorIndex)),
    removed_indexes: v.array(v.union(databaseIndex, searchIndex, vectorIndex)),
  }),
});

export const pushConfig = v.object({
  action: v.literal("push_config"),
  member_id: v.int64(),
  metadata: v.object({
    auth: v.object({
      added: v.array(v.string()),
      removed: v.array(v.string()),
    }),
    server_version: v.union(
      v.null(),
      v.object({
        previous_version: v.string(),
        next_version: v.string(),
      }),
    ),
    modules: v.object({
      added: v.array(v.string()),
      removed: v.array(v.string()),
    }),
    crons: v.optional(
      v.object({
        added: v.array(v.string()),
        updated: v.array(v.string()),
        deleted: v.array(v.string()),
      }),
    ),
    schema: v.optional(
      v.union(
        v.null(),
        v.object({
          previous_schema_id: v.union(v.id("_schemas"), v.null()),
          next_schema_id: v.union(v.id("_schemas"), v.null()),
          previous_schema: v.optional(v.union(v.string(), v.null())),
          next_schema: v.optional(v.union(v.string(), v.null())),
        }),
      ),
    ),
  }),
});

export const deploymentState = v.union(
  v.literal("paused"),
  v.literal("running"),
  v.literal("disabled"),
);

export const changeDeploymentState = v.object({
  action: v.literal("change_deployment_state"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    old_state: deploymentState,
    new_state: deploymentState,
  }),
});

export const clearTables = v.object({
  action: v.literal("clear_tables"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({}),
});

export const snapshotImportFormat = v.union(
  v.object({
    format: v.union(
      v.literal("csv"),
      v.literal("jsonl"),
      v.literal("json_array"),
    ),
    table: v.string(),
  }),
  v.object({
    format: v.literal("zip"),
  }),
);

export const snapshotImportMode = v.union(
  v.literal("RequireEmpty"),
  v.literal("Append"),
  v.literal("Replace"),
);

export const snapshotImport = v.object({
  action: v.literal("snapshot_import"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    table_names: v.array(v.string()),
    table_count: v.int64(),
    import_mode: snapshotImportMode,
    import_format: snapshotImportFormat,
  }),
});

const deploymentAuditLogTable = defineTable(
  v.union(
    createEnvironmentVariable,
    deleteEnvironmentVariable,
    updateEnvironmentVariable,
    replaceEnvironmentVariable,
    buildIndexes,
    pushConfig,
    changeDeploymentState,
    clearTables,
    snapshotImport,
  ),
);

export default deploymentAuditLogTable;
