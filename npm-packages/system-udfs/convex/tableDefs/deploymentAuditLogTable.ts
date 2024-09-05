import { defineTable } from "convex/server";
import { v } from "convex/values";
import { snapshotImportFormat, snapshotImportMode } from "./snapshotImport";

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

const indexConfigs = v.array(v.union(databaseIndex, searchIndex, vectorIndex));
export const buildIndexes = v.object({
  action: v.literal("build_indexes"),
  member_id: v.int64(),
  metadata: v.object({
    added_indexes: indexConfigs,
    removed_indexes: indexConfigs,
  }),
});

export const indexDiff = v.object({
  added_indexes: indexConfigs,
  removed_indexes: indexConfigs,
});

export const authDiff = v.object({
  added: v.array(v.string()),
  removed: v.array(v.string()),
});

const serverVersion = v.union(
  v.null(),
  v.object({
    previous_version: v.string(),
    next_version: v.string(),
  }),
);
const moduleDiff = v.object({
  added: v.array(v.string()),
  removed: v.array(v.string()),
});
export const cronDiffType = v.optional(
  v.object({
    added: v.array(v.string()),
    updated: v.array(v.string()),
    deleted: v.array(v.string()),
  }),
);
export const schemaDiffType = v.optional(
  v.union(
    v.null(),
    v.object({
      previous_schema_id: v.union(v.id("_schemas"), v.null()),
      next_schema_id: v.union(v.id("_schemas"), v.null()),
      previous_schema: v.optional(v.union(v.string(), v.null())),
      next_schema: v.optional(v.union(v.string(), v.null())),
    }),
  ),
);

export const pushConfig = v.object({
  action: v.literal("push_config"),
  member_id: v.int64(),
  metadata: v.object({
    auth: authDiff,
    server_version: serverVersion,
    modules: moduleDiff,
    crons: cronDiffType,
    schema: schemaDiffType,
  }),
});

export const componentDiff = v.object({
  diffType: v.object({
    type: v.union(
      v.literal("create"),
      v.literal("modify"),
      v.literal("unmount"),
      v.literal("remount"),
    ),
  }),
  indexDiff: indexDiff,
  udfConfigDiff: serverVersion,
  moduleDiff: moduleDiff,
  cronDiff: cronDiffType,
  schemaDiff: schemaDiffType,
});

export const pushConfigWithComponents = v.object({
  action: v.literal("push_config_with_components"),
  member_id: v.int64(),
  metadata: v.object({
    auth_diff: v.optional(authDiff),
    component_diffs: v.array(
      v.object({
        component_path: v.union(v.string(), v.null()),
        component_diff: componentDiff,
      }),
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
    pushConfigWithComponents,
    changeDeploymentState,
    clearTables,
    snapshotImport,
  ),
);

export default deploymentAuditLogTable;
