import { defineTable } from "convex/server";
import { v } from "convex/values";
import {
  snapshotImportFormat,
  snapshotImportMode,
  snapshotImportRequestor,
} from "./snapshotImport";

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

const updateCanonicalUrl = v.object({
  action: v.literal("update_canonical_url"),
  member_id: v.string(),
  metadata: v.object({
    request_destination: v.string(),
    url: v.string(),
  }),
});

const deleteCanonicalUrl = v.object({
  action: v.literal("delete_canonical_url"),
  member_id: v.string(),
  metadata: v.object({
    request_destination: v.string(),
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
  indexDiff: v.optional(indexDiff),
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
    message: v.optional(v.string()),
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

export const pauseDeployment = v.object({
  action: v.literal("pause_deployment"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({}),
});

export const unpauseDeployment = v.object({
  action: v.literal("unpause_deployment"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({}),
});

export const systemStopState = v.union(
  v.literal("none"),
  v.literal("disabled"),
  v.literal("resumable"),
  v.literal("suspended"),
);

export const changeSystemStopState = v.object({
  action: v.literal("change_system_stop_state"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    old_state: systemStopState,
    new_state: systemStopState,
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
    table_names: v.array(
      v.object({
        component: v.union(v.null(), v.string()),
        table_names: v.array(v.string()),
      }),
    ),
    table_count: v.int64(),
    import_mode: snapshotImportMode,
    import_format: snapshotImportFormat,
    requestor: snapshotImportRequestor,
    table_names_deleted: v.array(
      v.object({
        component: v.union(v.null(), v.string()),
        table_names: v.array(v.string()),
      }),
    ),
    table_count_deleted: v.int64(),
  }),
});

const componentMetadata = v.object({
  component_id: v.union(v.null(), v.string()),
  component: v.union(v.null(), v.string()),
});

const deleteScheduledJobsTable = v.object({
  action: v.literal("delete_scheduled_jobs_table"),
  member_id: v.union(v.int64(), v.null()),
  metadata: componentMetadata,
});

const deleteTables = v.object({
  action: v.literal("delete_tables"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    component_id: v.union(v.null(), v.string()),
    component: v.union(v.null(), v.string()),
    table_names: v.array(v.string()),
  }),
});

const deleteComponent = v.object({
  action: v.literal("delete_component"),
  member_id: v.union(v.int64(), v.null()),
  metadata: componentMetadata,
});

const cancelAllScheduledFunctions = v.object({
  action: v.literal("cancel_all_scheduled_functions"),
  member_id: v.union(v.int64(), v.null()),
  metadata: componentMetadata,
});

const cancelScheduledFunction = v.object({
  action: v.literal("cancel_scheduled_function"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    component_id: v.union(v.null(), v.string()),
    component: v.union(v.null(), v.string()),
    scheduled_function_id: v.string(),
    function_path: v.union(v.null(), v.string()),
  }),
});

const requestExport = v.object({
  action: v.literal("request_export"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    id: v.string(),
    component_id: v.union(v.null(), v.string()),
    component: v.union(v.null(), v.string()),
    format: v.string(),
    requestor: v.string(),
  }),
});

const cancelExport = v.object({
  action: v.literal("cancel_export"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    id: v.string(),
  }),
});

const setExportExpiration = v.object({
  action: v.literal("set_export_expiration"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    id: v.string(),
    expiration_ts_ms: v.int64(),
  }),
});

const createIntegration = v.object({
  action: v.literal("create_integration"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    id: v.string(),
    type: v.string(),
  }),
});

const updateIntegration = v.object({
  action: v.literal("update_integration"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    id: v.string(),
    type: v.string(),
  }),
});

const deleteIntegration = v.object({
  action: v.literal("delete_integration"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    id: v.string(),
    type: v.string(),
  }),
});

const addDocuments = v.object({
  action: v.literal("add_documents"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    component_id: v.union(v.null(), v.string()),
    component: v.union(v.null(), v.string()),
    table: v.string(),
    document_ids: v.array(v.string()),
  }),
});

const deleteDocuments = v.object({
  action: v.literal("delete_documents"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    component_id: v.union(v.null(), v.string()),
    component: v.union(v.null(), v.string()),
    table: v.string(),
    document_ids: v.array(v.string()),
  }),
});

const updateDocuments = v.object({
  action: v.literal("update_documents"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    component_id: v.union(v.null(), v.string()),
    component: v.union(v.null(), v.string()),
    table: v.string(),
    document_ids: v.array(v.string()),
  }),
});

const createTable = v.object({
  action: v.literal("create_table"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    component_id: v.union(v.null(), v.string()),
    component: v.union(v.null(), v.string()),
    table: v.string(),
  }),
});

const deleteFiles = v.object({
  action: v.literal("delete_files"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    component_id: v.union(v.null(), v.string()),
    component: v.union(v.null(), v.string()),
    storage_ids: v.array(v.string()),
  }),
});

const generateUploadUrl = v.object({
  action: v.literal("generate_upload_url"),
  member_id: v.union(v.int64(), v.null()),
  metadata: v.object({
    component_id: v.union(v.null(), v.string()),
    component: v.union(v.null(), v.string()),
  }),
});

const deploymentAuditLogTable = defineTable(
  v.union(
    createEnvironmentVariable,
    deleteEnvironmentVariable,
    updateEnvironmentVariable,
    replaceEnvironmentVariable,
    updateCanonicalUrl,
    deleteCanonicalUrl,
    buildIndexes,
    pushConfig,
    pushConfigWithComponents,
    changeDeploymentState,
    pauseDeployment,
    unpauseDeployment,
    changeSystemStopState,
    clearTables,
    snapshotImport,
    deleteScheduledJobsTable,
    deleteTables,
    deleteComponent,
    cancelAllScheduledFunctions,
    cancelScheduledFunction,
    requestExport,
    cancelExport,
    setExportExpiration,
    createIntegration,
    updateIntegration,
    deleteIntegration,
    addDocuments,
    deleteDocuments,
    updateDocuments,
    createTable,
    deleteFiles,
    generateUploadUrl,
  ),
);

export default deploymentAuditLogTable;
