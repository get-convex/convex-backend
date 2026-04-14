export const DATABASE_STORAGE_CATEGORIES = {
  document: {
    name: "Tables",
    color: "fill-chart-line-1",
  },
  index: {
    name: "Indexes",
    color: "fill-chart-line-2",
  },
};

export const FILE_STORAGE_CATEGORIES = {
  userFiles: {
    name: "User Files",
    color: "fill-chart-line-1",
  },
  cloudBackup: {
    name: "Cloud Backups",
    color: "fill-chart-line-2",
  },
};

export const BANDWIDTH_CATEGORIES = {
  egress: {
    name: "Reads",
    color: "fill-chart-line-1",
    backgroundColor: "bg-background-success",
  },
  ingress: {
    name: "Writes",
    // eslint-disable-next-line no-restricted-syntax
    color: "fill-chart-line-2",
    // eslint-disable-next-line no-restricted-syntax
    backgroundColor: "bg-background-error",
  },
};

export const FILE_BANDWIDTH_CATEGORIES = {
  servingEgress: {
    name: "Serving Reads",
    color: "fill-chart-line-1",
  },
  servingIngress: {
    name: "Serving Writes",
    color: "fill-chart-line-2",
  },
  userFunctionEgress: {
    name: "User Function Reads",
    color: "fill-chart-line-3",
  },
  userFunctionIngress: {
    name: "User Function Writes",
    color: "fill-chart-line-4",
  },
  cloudRestore: {
    name: "Cloud Restore",
    color: "fill-chart-line-5",
  },
  cloudBackup: {
    name: "Cloud Backup",
    color: "fill-chart-line-6",
  },
  snapshotExport: {
    name: "Snapshot Export",
    color: "fill-chart-line-7",
  },
  snapshotImport: {
    name: "Snapshot Import",
    color: "fill-chart-line-8",
  },
};

export const TAG_CATEGORIES = {
  query: {
    name: "Queries",
    color: "fill-chart-line-1",
  },
  mutation: {
    name: "Mutations",
    color: "fill-chart-line-2",
  },
  action: {
    name: "Actions",
    color: "fill-chart-line-3",
  },
  http_action: {
    name: "HTTP Actions",
    color: "fill-chart-line-4",
  },
  storage_api: {
    name: "Storage API",
    color: "fill-chart-line-5",
  },
};

export const DATA_EGRESS_CATEGORIES_SELF_SERVE = {
  servingEgress: {
    name: "Serving Reads",
    color: "fill-chart-line-1",
  },
  userFunctionEgress: {
    name: "User Function Reads",
    color: "fill-chart-line-2",
  },
  backupRestore: {
    name: "Backup / Restore",
    color: "fill-chart-line-3",
  },
};

export const DATA_EGRESS_CATEGORY_RENAMES_SELF_SERVE: Record<string, string> = {
  backup: "backupRestore",
  restore: "backupRestore",
};

export const DATA_EGRESS_CATEGORIES = {
  fetchEgress: {
    name: "Fetch Egress",
    color: "fill-chart-line-1",
  },
  logStream: {
    name: "Log Streams",
    color: "fill-chart-line-2",
  },
  servingEgress: {
    name: "Serving Reads",
    color: "fill-chart-line-3",
  },
  userFunctionEgress: {
    name: "User Function Reads",
    color: "fill-chart-line-4",
  },
  backupRestore: {
    name: "Backup / Restore",
    color: "fill-chart-line-5",
  },
};

export const DATA_EGRESS_CATEGORY_RENAMES: Record<string, string> = {
  backup: "backupRestore",
  restore: "backupRestore",
};

export const SEARCH_STORAGE_CATEGORIES = {
  textSearch: {
    name: "Text Search",
    color: "fill-chart-line-1",
  },
  vector: {
    name: "Vector",
    color: "fill-chart-line-2",
  },
};

export const SEARCH_QUERIES_CATEGORIES = {
  textSearch: {
    name: "Text Search",
    color: "fill-chart-line-1",
  },
  vectorSearch: {
    name: "Vector Search",
    color: "fill-chart-line-2",
  },
};

export const DATABASE_IO_CATEGORIES = {
  egress: {
    name: "Reads",
    color: "fill-chart-line-1",
  },
  ingress: {
    name: "Writes",
    // eslint-disable-next-line no-restricted-syntax
    color: "fill-chart-line-2",
  },
};

export const COMPUTE_CATEGORIES_SELF_SERVE = {
  actionConvex: {
    name: "Action",
    // eslint-disable-next-line no-restricted-syntax
    color: "fill-chart-line-1",
  },
  actionNode: {
    name: "Action (Node)",
    color: "fill-chart-line-2",
  },
};

export const COMPUTE_CATEGORIES = {
  queryMutation: {
    name: "Query/Mutation (Dedicated)",
    color: "fill-chart-line-1",
  },
  actionConvex: {
    name: "Action",
    // eslint-disable-next-line no-restricted-syntax
    color: "fill-chart-line-2",
  },
  actionNode: {
    name: "Action (Node)",
    color: "fill-chart-line-3",
  },
};

const DEPLOYMENT_CLASS_COLORS: Record<string, string> = {
  s16: "fill-chart-line-1",
  s256: "fill-chart-line-2",
  d1024: "fill-chart-line-3",
};

export const DEPLOYMENT_CLASS_CATEGORIES: Record<
  string,
  { name: string; color: string }
> = Object.fromEntries(
  Object.entries(DEPLOYMENT_CLASS_COLORS).map(([key, color]) => [
    key,
    { name: key.toUpperCase(), color },
  ]),
);

export const CATEGORY_RENAMES = {
  uncached_query: "query",
  cached_query: "query",
};
