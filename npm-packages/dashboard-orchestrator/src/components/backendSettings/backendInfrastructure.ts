export const PROVISIONING_MODE_OVERRIDE_KEY =
  "CONVEX_ORCHESTRATOR_PROVISIONING_MODE";
export const DATABASE_MODE_OVERRIDE_KEY = "CONVEX_ORCHESTRATOR_DATABASE_MODE";
export const STORAGE_MODE_OVERRIDE_KEY = "CONVEX_ORCHESTRATOR_STORAGE_MODE";

export type ProvisioningMode = "default" | "volume-sqlite" | "sidecar";
export type DatabaseMode = "sqlite" | "sidecar" | "external";
export type StorageMode = "local" | "sidecar" | "external";

export type DatabaseDraft =
  | { kind: "sidecar" }
  | { kind: "sqlite" }
  | { kind: "postgres"; url: string }
  | { kind: "mysql"; url: string };

export type StorageBucketsDraft = {
  exports: string;
  snapshotImports: string;
  modules: string;
  files: string;
  search: string;
};

export type ObjectStorageDraft =
  | { kind: "sidecar" }
  | { kind: "local" }
  | {
      kind: "s3" | "s3-compatible";
      region: string;
      endpointUrl?: string;
      accessKeyId: string;
      secretAccessKey: string;
      forcePathStyle?: boolean;
      disableSse?: boolean;
      disableChecksums?: boolean;
      buckets: StorageBucketsDraft;
    };

export type BackendInfrastructureDraft = {
  database: DatabaseDraft;
  storage: ObjectStorageDraft;
};

export const DEFAULT_INFRASTRUCTURE: BackendInfrastructureDraft = {
  database: { kind: "sidecar" },
  storage: { kind: "sidecar" },
};

export const DEFAULT_BUCKETS: StorageBucketsDraft = {
  exports: "convex-exports",
  snapshotImports: "convex-snapshot-imports",
  modules: "convex-modules",
  files: "convex-files",
  search: "convex-search",
};

export function infrastructureOverrides(draft: BackendInfrastructureDraft): {
  provisioningMode: ProvisioningMode;
  overrides: Record<string, string>;
} {
  const overrides: Record<string, string> = {};
  const usesSidecars =
    draft.database.kind === "sidecar" || draft.storage.kind === "sidecar";
  const provisioningMode: ProvisioningMode = usesSidecars
    ? "sidecar"
    : "volume-sqlite";
  const databaseMode = databaseModeForDraft(draft.database);
  const storageMode = storageModeForDraft(draft.storage);

  overrides[PROVISIONING_MODE_OVERRIDE_KEY] = provisioningMode;
  overrides[DATABASE_MODE_OVERRIDE_KEY] = databaseMode;
  overrides[STORAGE_MODE_OVERRIDE_KEY] = storageMode;

  if (draft.database.kind === "postgres") {
    setIfPresent(overrides, "POSTGRES_URL", draft.database.url);
  } else if (draft.database.kind === "mysql") {
    setIfPresent(overrides, "MYSQL_URL", draft.database.url);
  }

  if (draft.storage.kind === "s3" || draft.storage.kind === "s3-compatible") {
    setIfPresent(overrides, "AWS_REGION", draft.storage.region);
    setIfPresent(overrides, "AWS_ACCESS_KEY_ID", draft.storage.accessKeyId);
    setIfPresent(
      overrides,
      "AWS_SECRET_ACCESS_KEY",
      draft.storage.secretAccessKey,
    );
    setIfPresent(overrides, "S3_ENDPOINT_URL", draft.storage.endpointUrl);
    if (draft.storage.forcePathStyle) {
      overrides.AWS_S3_FORCE_PATH_STYLE = "true";
    }
    if (draft.storage.disableSse) {
      overrides.AWS_S3_DISABLE_SSE = "true";
    }
    if (draft.storage.disableChecksums) {
      overrides.AWS_S3_DISABLE_CHECKSUMS = "true";
    }
    setIfPresent(
      overrides,
      "S3_STORAGE_EXPORTS_BUCKET",
      draft.storage.buckets.exports,
    );
    setIfPresent(
      overrides,
      "S3_STORAGE_SNAPSHOT_IMPORTS_BUCKET",
      draft.storage.buckets.snapshotImports,
    );
    setIfPresent(
      overrides,
      "S3_STORAGE_MODULES_BUCKET",
      draft.storage.buckets.modules,
    );
    setIfPresent(
      overrides,
      "S3_STORAGE_FILES_BUCKET",
      draft.storage.buckets.files,
    );
    setIfPresent(
      overrides,
      "S3_STORAGE_SEARCH_BUCKET",
      draft.storage.buckets.search,
    );
  }

  return { provisioningMode, overrides };
}

function databaseModeForDraft(database: DatabaseDraft): DatabaseMode {
  if (database.kind === "sidecar") {
    return "sidecar";
  }
  if (database.kind === "sqlite") {
    return "sqlite";
  }
  return "external";
}

function storageModeForDraft(storage: ObjectStorageDraft): StorageMode {
  if (storage.kind === "sidecar") {
    return "sidecar";
  }
  if (storage.kind === "local") {
    return "local";
  }
  return "external";
}

function setIfPresent(
  target: Record<string, string>,
  key: string,
  value: string | undefined,
) {
  const trimmed = value?.trim();
  if (trimmed) {
    target[key] = trimmed;
  }
}
