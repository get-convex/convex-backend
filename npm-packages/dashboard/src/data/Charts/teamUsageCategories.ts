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
  ingress: {
    name: "Writes",
    // eslint-disable-next-line no-restricted-syntax
    color: "fill-chart-line-1",
    // eslint-disable-next-line no-restricted-syntax
    backgroundColor: "bg-background-error",
  },
  egress: {
    name: "Reads",
    color: "fill-chart-line-2",
    backgroundColor: "bg-background-success",
  },
};

export const FILE_BANDWIDTH_CATEGORIES = {
  servingIngress: {
    name: "Serving Writes",
    color: "fill-chart-line-1",
  },
  servingEgress: {
    name: "Serving Reads",
    color: "fill-chart-line-2",
  },
  userFunctionIngress: {
    name: "User Function Writes",
    color: "fill-chart-line-3",
  },
  userFunctionEgress: {
    name: "User Function Reads",
    color: "fill-chart-line-4",
  },
  cloudBackup: {
    name: "Cloud Backup",
    color: "fill-chart-line-5",
  },
  cloudRestore: {
    name: "Cloud Restore",
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

export const CATEGORY_RENAMES = {
  uncached_query: "query",
  cached_query: "query",
};
